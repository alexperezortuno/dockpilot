use bollard::{
    Docker,
    exec::{CreateExecOptions, ResizeExecOptions, StartExecOptions, StartExecResults},
    query_parameters::ListContainersOptionsBuilder,
};
use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;
use tokio::time::{Duration, timeout};

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

#[tokio::test]
#[ignore = "requires a running container and Docker Engine"]
async fn interactive_exec_round_trip_resize_and_exit_work() {
    let container = std::env::var("DOCKPILOT_EXEC_CONTAINER")
        .expect("set DOCKPILOT_EXEC_CONTAINER to a running container ID");
    let docker = Docker::connect_with_local_defaults().expect("connect to Docker Engine");
    let created = docker
        .create_exec(
            &container,
            CreateExecOptions {
                attach_stdin: Some(true),
                attach_stdout: Some(true),
                attach_stderr: Some(true),
                tty: Some(true),
                cmd: Some(vec!["/bin/sh"]),
                ..Default::default()
            },
        )
        .await
        .expect("create exec");
    let started = docker
        .start_exec(
            &created.id,
            Some(StartExecOptions {
                detach: false,
                tty: true,
                output_capacity: Some(64 * 1024),
            }),
        )
        .await
        .expect("start exec");
    let StartExecResults::Attached {
        mut output,
        mut input,
    } = started
    else {
        panic!("exec was not attached");
    };

    docker
        .resize_exec(
            &created.id,
            ResizeExecOptions {
                width: 40,
                height: 10,
            },
        )
        .await
        .expect("resize exec");
    input
        .write_all(b"printf 'dockpilot-exec-ok\\n'; exit\n")
        .await
        .expect("write to exec stdin");

    let mut received = Vec::new();
    while let Some(chunk) = timeout(Duration::from_secs(5), output.next())
        .await
        .expect("exec output timeout")
    {
        received.extend_from_slice(&chunk.expect("read exec output").into_bytes());
    }
    assert!(
        String::from_utf8_lossy(&received).contains("dockpilot-exec-ok"),
        "output was {:?}",
        String::from_utf8_lossy(&received)
    );
    let inspected = docker
        .inspect_exec(&created.id)
        .await
        .expect("inspect exec");
    assert_eq!(inspected.exit_code, Some(0));
}

#[tokio::test]
#[ignore = "requires a running Docker Engine"]
async fn exec_rejects_stopped_and_missing_containers() {
    let docker = Docker::connect_with_local_defaults().expect("connect to Docker Engine");
    let stopped = std::env::var("DOCKPILOT_EXEC_STOPPED_CONTAINER")
        .expect("set DOCKPILOT_EXEC_STOPPED_CONTAINER");
    let error = docker
        .create_exec(
            &stopped,
            CreateExecOptions {
                attach_stdout: Some(true),
                tty: Some(true),
                cmd: Some(vec!["/bin/sh"]),
                ..Default::default()
            },
        )
        .await
        .expect_err("creating exec in a stopped container should fail");
    assert!(!error.to_string().is_empty());

    let error = docker
        .create_exec(
            "dockpilot-container-that-does-not-exist",
            CreateExecOptions {
                attach_stdout: Some(true),
                tty: Some(true),
                cmd: Some(vec!["/bin/sh"]),
                ..Default::default()
            },
        )
        .await
        .expect_err("missing container should fail");
    assert!(!error.to_string().is_empty());
}
