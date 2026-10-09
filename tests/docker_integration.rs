use bollard::{Docker, query_parameters::ListContainersOptionsBuilder};

#[tokio::test]
#[ignore = "requires a reachable Docker Engine"]
async fn docker_engine_ping_and_container_listing_work() {
    let docker = Docker::connect_with_local_defaults().expect("connect to Docker Engine");
    docker.ping().await.expect("ping Docker Engine");
    docker
        .list_containers(Some(ListContainersOptionsBuilder::new().all(true).build()))
        .await
        .expect("list Docker containers");
}
