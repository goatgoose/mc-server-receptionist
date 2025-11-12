use async_trait::async_trait;
use aws_config::BehaviorVersion;
use aws_sdk_ec2::config::http::HttpResponse;
use aws_sdk_ec2::operation::describe_instances::DescribeInstancesOutput;
use aws_sdk_ec2::operation::start_instances::{StartInstancesError, StartInstancesOutput};
use aws_sdk_ec2::types::{Filter, Instance, InstanceState, InstanceStateName};
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

    pub async fn describe_instance(&self) -> Result<Instance, aws_sdk_ec2::Error> {
        let filter = Filter::builder()
            .name("tag:Name")
            .values(&self.instance_name)
            .build();
        let description = self.ec2.describe_instances().filters(filter).send().await?;
        let reservation = description.reservations().get(0).unwrap();
        let instance = reservation.instances().get(0).unwrap();
        Ok(instance.clone())
    }

    async fn get_public_ip(instance: &Instance) -> Option<String> {
        for network_interface in instance.network_interfaces() {
            if let Some(association) = network_interface.association() {
                if let Some(public_ip) = association.public_ip() {
                    return Some(public_ip.to_string());
                }
            }
        }

        None
    }

    async fn get_transfer(&self, instance: &Instance) -> Option<Transfer> {
        let state = instance.state().unwrap().name().unwrap();
        if let InstanceStateName::Running = state {
            let public_ip = InstanceManager::get_public_ip(instance).await.unwrap();
            return Some(Transfer {
                hostname: public_ip,
                port: self.mc_target_port,
            });
        }

        println!("Unable to get Transfer: instance not running.");
        None
    }

    async fn try_launch_instance(&self, instance: &Instance) {
        let state = instance.state().unwrap().name().unwrap();
        if let InstanceStateName::Stopped = state {
            let instance_id = instance.instance_id().unwrap();
            println!("launching {}...", instance_id);
            if let Err(e) = self.ec2.start_instances().instance_ids(instance_id).send().await {
                println!("unable to start instance: {}", e);
            }
        } else {
            println!("unable to start instance in state {}", state);
        }
    }
}

#[async_trait]
impl TransferHandler for InstanceManager {
    async fn on_join(&self, login_start: &LoginStart) -> Option<Transfer> {
        println!("{} joined!", login_start.username);

        let instance = self.describe_instance().await.unwrap();
        if let Some(transfer) = self.get_transfer(&instance).await {
            Some(transfer)
        } else {
            self.try_launch_instance(&instance).await;
            None
        }
    }

    async fn on_transfer_ready(&self) -> Option<Transfer> {
        let instance = self.describe_instance().await.unwrap();
        self.get_transfer(&instance).await
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
        instance_manager.describe_instance().await.unwrap();

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
