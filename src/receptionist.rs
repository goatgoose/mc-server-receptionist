use async_trait::async_trait;
use aws_config::BehaviorVersion;
use aws_sdk_ec2::operation::describe_instances::DescribeInstancesOutput;
use aws_sdk_ec2::types::{Filter, Instance};
use crate::connection::{Connection, TransferHandler, LoginStart, Transfer};
use tokio::io;
use tokio::net::{TcpListener, TcpStream};

#[derive(Clone)]
struct InstanceManager {
    instance_name: String,
    mc_target_port: u16,
    ec2: aws_sdk_ec2::Client,
}

impl InstanceManager {
    pub async fn new(
        instance_name: String,
        mc_target_port: u16,
    ) -> InstanceManager {
        let config = aws_config::defaults(BehaviorVersion::latest())
            .load()
            .await;
        let ec2 = aws_sdk_ec2::Client::new(&config);

        InstanceManager {
            instance_name,
            mc_target_port,
            ec2,
        }
    }

    async fn describe_instance(&self) -> Result<Instance, aws_sdk_ec2::Error> {
        let filter = Filter::builder()
            .name("tag:Name")
            .values(&self.instance_name)
            .build();
        let description = self.ec2.describe_instances().filters(filter).send().await?;
        let reservation = description.reservations().get(0).unwrap();
        let instance = reservation.instances().get(0).unwrap();
        Ok(instance.clone())
    }

    async fn get_public_ip(&self) -> Option<String> {
        if let Ok(instance) = self.describe_instance().await {
            for network_interface in instance.network_interfaces() {
                if let Some(association) = network_interface.association() {
                    if let Some(public_ip) = association.public_ip() {
                        return Some(public_ip.to_string());
                    }
                }
            }
        }

        None
    }

    async fn get_transfer(&self) -> Option<Transfer> {
        match self.get_public_ip().await {
            Some(public_ip) => Some(Transfer {
                hostname: public_ip,
                port: self.mc_target_port,
            }),
            None => None,
        }
    }
}

#[async_trait]
impl TransferHandler for InstanceManager {
    async fn on_join(&self, login_start: &LoginStart) -> Option<Transfer> {
        println!("{} joined!", login_start.username);
        self.get_transfer().await
    }

    async fn on_transfer_ready(&self) -> Option<Transfer> {
        self.get_transfer().await
    }
}

pub struct Receptionist {
    instance_manager: InstanceManager,
}

impl Receptionist {
    pub async fn new(
        target_instance_name: String,
        mc_target_port: u16,
        mc_target_motd: String,
    ) -> Receptionist {
        let instance_manager = InstanceManager::new(
            target_instance_name,
            mc_target_port,
        ).await;

        Receptionist {
            instance_manager,
        }
    }

    pub async fn listen(self, addr: &str) -> io::Result<()> {
        let listener = TcpListener::bind(addr).await?;
        println!("Listening on: {}", addr);

        loop {
            let (stream, addr) = listener.accept().await?;
            println!("Accepted connection from: {}", &addr);

            let instance_manager = self.instance_manager.clone();

            tokio::spawn(async move {
                let mut connection = Connection::new(stream, instance_manager);
                match connection.process().await {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("error: {e}");
                        return;
                    }
                }
                println!("process complete for {}", &addr);
            });
        }
    }
}
