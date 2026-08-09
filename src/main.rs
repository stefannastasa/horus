use std::process::exit;

use bollard::{Docker, query_parameters::ListContainersOptionsBuilder};

#[tokio::main]
async fn main() -> Result<(), ()> {
    println!("horus started, pid {}", std::process::id());
    let docker_conn = match Docker::connect_with_defaults() {
        Err(e) => {
            println!(
                "Failed to init connection to the docker daemon {}: . Exiting...",
                e
            );
            exit(1);
        }

        Ok(conn) => conn,
    };

    let options = ListContainersOptionsBuilder::default().all(true).build();
    let containers = match docker_conn.list_containers(Some(options)).await {
        Err(e) => {
            println!("Failed to list containers {}: . Exiting...", e);
            exit(1);
        }

        Ok(containers) => containers,
    };

    for c in containers {
        println!("{:?} {:?}", c.names, c.state);
    }
    Ok(())
}
