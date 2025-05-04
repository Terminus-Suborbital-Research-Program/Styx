use get_if_addrs::get_if_addrs;
use std::{thread, time::Duration};

use log::warn;
use ureq::Agent;

pub fn ping_thread() -> ! {
    let token = std::env::var("PALANTIR_BEARER_TOKEN").expect("Failed to get bearer token");
    let client: Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .build()
        .into();

    let url = "https://lucasthelen.usw-16.palantirfoundry.com/api/v2/highScale/streams/datasets/ri.foundry.main.dataset.475f3da7-8c59-4f02-b32e-31a6fcaa8e30/streams/master/publishRecords?preview=true";

    loop {
        match get_if_addrs() {
            Ok(ifs) => {
                for iface in ifs {
                    if iface.name == "wlan0" {
                        if let std::net::IpAddr::V4(ipv4) = iface.ip() {
                            if !ipv4.is_loopback() {
                                let body = serde_json::json!({
                                    "records": [{
                                        "timestamp": 1744677939051u64,
                                        "field": "jupiter_ip",
                                        "numeric": false,
                                        "string": format!("{}", ipv4),
                                        "float": 0
                                    }]
                                });

                                let response = client
                                    .post(url)
                                    .header("Authorization", &format!("Bearer {}", token))
                                    .send_json(&body);

                                match response {
                                    Ok(_) => {}
                                    Err(e) => {
                                        warn!("Error pushing IP to palantir: {:?}", e);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            Err(_) => {
                warn!("No IP Interface");
            }
        }

        thread::sleep(Duration::from_secs(10));
    }
}
