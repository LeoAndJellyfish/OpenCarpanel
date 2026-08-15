use std::{
    error::Error,
    net::TcpListener,
    path::Path,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _kill_result = self.0.kill();
        let _wait_result = self.0.wait();
    }
}

#[test]
fn two_headless_entry_points_report_shared_instance_ownership() -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let data = temp.path().join("data");
    let runtime = temp.path().join("runtime");
    let mut first = ChildGuard(host_command(&data, &runtime).spawn()?);
    wait_for_owner(&mut first.0, &runtime)?;

    let second = host_command(&data, &runtime).output()?;
    assert!(!second.status.success());
    let stderr = String::from_utf8_lossy(&second.stderr);
    assert!(
        stderr.contains("OpenSimDash is already running"),
        "{stderr}"
    );
    assert!(stderr.contains("headless"), "{stderr}");
    Ok(())
}

#[test]
fn third_party_port_conflict_is_not_misreported_as_an_instance() -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let occupied = TcpListener::bind("127.0.0.1:0")?;
    let port = occupied.local_addr()?.port();
    let output = host_command(&temp.path().join("data"), &temp.path().join("runtime"))
        .env("OPENSIMDASH_HTTP_BIND", format!("127.0.0.1:{port}"))
        .output()?;

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to bind HTTP"), "{stderr}");
    assert!(!stderr.contains("already running"), "{stderr}");
    Ok(())
}

fn host_command(data: &Path, runtime: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_opensimdash-host"));
    command
        .env("OPENSIMDASH_DATA_DIR", data)
        .env("OPENSIMDASH_RUNTIME_DIR", runtime)
        .env("OPENSIMDASH_HTTP_BIND", "127.0.0.1:0")
        .env("OPENSIMDASH_UDP_BIND", "127.0.0.1:0")
        .env("RUST_LOG", "off")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

fn wait_for_owner(child: &mut Child, runtime: &Path) -> Result<(), Box<dyn Error>> {
    let owner = runtime.join("instance-owner.json");
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if owner.is_file() {
            return Ok(());
        }
        if let Some(status) = child.try_wait()? {
            return Err(format!("first Host exited before acquiring its lock: {status}").into());
        }
        thread::sleep(Duration::from_millis(20));
    }
    Err("first Host did not publish instance ownership within five seconds".into())
}
