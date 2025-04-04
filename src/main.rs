use std::{collections::HashMap, sync::Arc, time::Duration};

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use tokio::sync::Mutex;

const DOMAIN_LABEL: &str = "_alacrite._tcp.local.";
const INSTANCE_LABEL: &str = "Alacrite";

struct AlacriteService {
    daemon: ServiceDaemon,
    service_info: ServiceInfo,
}

impl AlacriteService {
    fn new(port: u16) -> Result<Self, Box<dyn std::error::Error>> {
        let local_ip = local_ip_address::local_ip()?;
        let daemon = ServiceDaemon::new()?;

        let service_info = ServiceInfo::new(
            DOMAIN_LABEL,
            INSTANCE_LABEL,
            &format!("{}.local.", local_ip),
            local_ip.to_string(),
            port,
            vec![],
        )?;

        daemon.register(service_info.clone())?;

        Ok(Self {
            daemon,
            service_info,
        })
    }

    async fn start_listening(
        &self,
        discovered_services: Arc<Mutex<HashMap<String, String>>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let receiver = self.daemon.browse(DOMAIN_LABEL)?;

        println!("Starting Alacrite service listener...");

        loop {
            let receiver_clone = receiver.clone();

            let event = tokio::task::spawn_blocking(move || receiver_clone.recv()).await?;

            if let Ok(event) = event {
                match event {
                    ServiceEvent::ServiceResolved(info) => {
                        let service_name = info.get_fullname();
                        let service_host = info.get_hostname();
                        let service_port = info.get_port().to_string();

                        let local_ip = local_ip_address::local_ip()?.to_string();
                        let Some(service_host_ip) = service_host
                            .split_once(".local.")
                            .map(|s| s.0)
                            .filter(|ip| *ip != local_ip)
                        else {
                            continue;
                        };

                        println!("New Alacrite service discovered:");
                        println!("  Name: {}", service_name);
                        println!("  Host: {:?}", service_host_ip);
                        println!("  Port: {}", service_port);

                        let mut services = discovered_services.lock().await;
                        services.insert(service_name.to_string(), service_host.to_string());
                    }
                    ServiceEvent::ServiceRemoved(name, _) => {
                        println!("Alacrite service removed: {}", name);
                    }
                    _ => {}
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = AlacriteService::new(8080)?;

    let discovered_services: Arc<Mutex<HashMap<String, String>>> =
        Arc::new(Mutex::new(HashMap::new()));

    tokio::spawn({
        let discovered_services = discovered_services.clone();
        async move {
            if let Err(e) = service.start_listening(discovered_services).await {
                eprintln!("Error occurred while listening for services: {}", e);
            }
        }
    });

    println!("Alacrite service registered and running...");
    println!("Press Ctrl+C to exit");

    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
