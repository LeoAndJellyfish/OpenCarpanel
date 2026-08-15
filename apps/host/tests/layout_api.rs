use std::{error::Error, fs, io, time::Duration};

use futures_util::{SinkExt, StreamExt};
use opencarpanel_config::{LayoutDocument, LayoutId, LayoutRepository, MAX_LAYOUT_BYTES};
use opencarpanel_host::{RunningHost, spawn_host_with_layout_repository};
use serde_json::{Value, json};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream, UdpSocket},
};
use tokio_tungstenite::{connect_async, tungstenite::Message};

struct TestHost {
    running: RunningHost,
    repository: LayoutRepository,
    _data: tempfile::TempDir,
}

impl TestHost {
    async fn start() -> Result<Self, Box<dyn Error>> {
        let data = tempfile::tempdir()?;
        let repository = LayoutRepository::new(data.path());
        let running = spawn_host_with_layout_repository(
            TcpListener::bind("127.0.0.1:0").await?,
            UdpSocket::bind("127.0.0.1:0").await?,
            repository.clone(),
        )?;
        Ok(Self {
            running,
            repository,
            _data: data,
        })
    }

    async fn session(&self) -> Result<String, Box<dyn Error>> {
        let token = self
            .running
            .issue_pairing_token(Duration::from_secs(30))
            .await?;
        let (mut socket, _) =
            connect_async(format!("ws://{}/api/v1/ws", self.running.http_address())).await?;
        socket
            .send(Message::Text(
                json!({
                    "v": 1,
                    "type": "hello",
                    "pairingToken": token,
                    "lastEventSeq": 0,
                    "snapshotHz": 60
                })
                .to_string()
                .into(),
            ))
            .await?;
        let session = loop {
            let message = tokio::time::timeout(Duration::from_secs(2), socket.next())
                .await?
                .ok_or_else(|| io::Error::other("WebSocket closed before hello"))??;
            if let Message::Text(text) = message {
                let value: Value = serde_json::from_str(text.as_ref())?;
                if value["type"] == "hello" {
                    break value["deviceSession"]
                        .as_str()
                        .ok_or_else(|| io::Error::other("missing device session"))?
                        .to_owned();
                }
            }
        };
        socket.close(None).await?;
        Ok(session)
    }
}

#[derive(Debug)]
struct HttpResponse {
    status: u16,
    body: Vec<u8>,
}

impl HttpResponse {
    fn json(&self) -> Result<Value, serde_json::Error> {
        serde_json::from_slice(&self.body)
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn built_in_games_receive_independent_default_layouts() -> Result<(), Box<dyn Error>> {
    let host = TestHost::start().await?;
    let address = host.running.http_address();
    let session = host.session().await?;
    let authorization = format!("Bearer {session}");

    for (id, name, accent, tachometer_x, widget_count, supplemental) in [
        ("game-f1-24", "F1 24 Default", "#d9ff43", 0, 6, "core.race"),
        ("game-f1-25", "F1 25 Default", "#42e8ff", 0, 6, "core.race"),
        ("game-ets2", "ETS2 Default", "#ffbd45", 5, 5, "core.route"),
        ("game-ats", "ATS Default", "#ff6a3d", 5, 5, "core.route"),
    ] {
        let response = request(
            address,
            "GET",
            &format!("/api/v1/layouts/{id}"),
            &[("Authorization", authorization.as_str())],
            &[],
        )
        .await?;
        assert_eq!(response.status, 200);
        let document = &response.json()?["document"];
        assert_eq!(document["id"], id);
        assert_eq!(document["name"], name);
        assert_eq!(document["revision"], 1);
        assert_eq!(document["theme"]["accent"], accent);
        assert_eq!(
            document["widgets"].as_array().map(Vec::len),
            Some(widget_count)
        );
        assert!(document["widgets"].as_array().is_some_and(|widgets| {
            widgets
                .iter()
                .any(|widget| widget["componentType"] == supplemental)
        }));
        assert_eq!(
            document["widgets"][0]["placements"]["phonePortrait"]["x"],
            tachometer_x
        );
    }

    let unknown = request(
        address,
        "GET",
        "/api/v1/layouts/game-future",
        &[("Authorization", authorization.as_str())],
        &[],
    )
    .await?;
    assert_eq!(unknown.status, 404);

    host.running.shutdown().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn untouched_v0_3_builtin_layouts_upgrade_without_overwriting_custom_revisions()
-> Result<(), Box<dyn Error>> {
    let host = TestHost::start().await?;
    let old = json!({
        "schemaVersion": 1,
        "revision": 0,
        "id": "game-f1-25",
        "name": "F1 25 Electric Grid",
        "widgets": [
            {"instanceId": "tachometer", "componentType": "core.tachometer", "placements": {}, "settings": {"fallbackRpmMax": 12000}},
            {"instanceId": "gear", "componentType": "core.gear", "placements": {}, "settings": {}},
            {"instanceId": "speed", "componentType": "core.speed", "placements": {}, "settings": {"unit": "km/h"}},
            {"instanceId": "status", "componentType": "core.status", "placements": {}, "settings": {}}
        ],
        "theme": {"background": "#061015", "foreground": "#eefcff", "accent": "#42e8ff", "warning": "#ff5e6c"}
    });
    let old_document: LayoutDocument = serde_json::from_value(old.clone())?;
    host.repository.save(&old_document, 0)?;
    let session = host.session().await?;
    let authorization = format!("Bearer {session}");

    let upgraded = request(
        host.running.http_address(),
        "GET",
        "/api/v1/layouts/game-f1-25",
        &[("Authorization", authorization.as_str())],
        &[],
    )
    .await?
    .json()?;
    assert_eq!(upgraded["document"]["revision"], 2);
    assert_eq!(
        upgraded["document"]["widgets"].as_array().map(Vec::len),
        Some(6)
    );
    assert!(
        upgraded["document"]["widgets"]
            .as_array()
            .is_some_and(|widgets| widgets
                .iter()
                .any(|widget| widget["componentType"] == "core.tyres"))
    );

    let custom_id = LayoutId::new("game-f1-24")?;
    let mut custom: LayoutDocument = serde_json::from_value(json!({
        "schemaVersion": 1,
        "revision": 0,
        "id": "game-f1-24",
        "name": "F1 24 Trackside",
        "widgets": old["widgets"].clone(),
        "theme": {"background": "#07090c", "foreground": "#f2f0e9", "accent": "#d9ff43", "warning": "#ff4b3e"}
    }))?;
    custom = host.repository.save(&custom, 0)?;
    custom.set_name("My compact F1 layout");
    host.repository.save(&custom, 1)?;

    let preserved = request(
        host.running.http_address(),
        "GET",
        "/api/v1/layouts/game-f1-24",
        &[("Authorization", authorization.as_str())],
        &[],
    )
    .await?
    .json()?;
    assert_eq!(preserved["document"]["id"], custom_id.as_str());
    assert_eq!(preserved["document"]["revision"], 2);
    assert_eq!(preserved["document"]["name"], "My compact F1 layout");
    assert_eq!(
        preserved["document"]["widgets"].as_array().map(Vec::len),
        Some(4)
    );

    host.running.shutdown().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_saved_v0_1_0_default_is_copied_into_the_first_game_layout() -> Result<(), Box<dyn Error>>
{
    let host = TestHost::start().await?;
    let address = host.running.http_address();
    let session = host.session().await?;
    let authorization = format!("Bearer {session}");
    let initial = request(
        address,
        "GET",
        "/api/v1/layouts/default",
        &[("Authorization", authorization.as_str())],
        &[],
    )
    .await?
    .json()?["document"]
        .clone();

    let untouched_truck = request(
        address,
        "GET",
        "/api/v1/layouts/game-ets2",
        &[("Authorization", authorization.as_str())],
        &[],
    )
    .await?
    .json()?;
    assert_eq!(untouched_truck["document"]["theme"]["accent"], "#ffbd45");

    let mut legacy = initial;
    legacy["name"] = json!("My v0.1.0 Layout");
    legacy["theme"]["accent"] = json!("#123456");
    let saved = request(
        address,
        "PUT",
        "/api/v1/layouts/default",
        &[
            ("Authorization", authorization.as_str()),
            ("Content-Type", "application/json"),
        ],
        &serde_json::to_vec(&legacy)?,
    )
    .await?;
    assert_eq!(saved.status, 200);
    assert_eq!(saved.json()?["document"]["revision"], 2);

    let migrated = request(
        address,
        "GET",
        "/api/v1/layouts/game-f1-24",
        &[("Authorization", authorization.as_str())],
        &[],
    )
    .await?;
    assert_eq!(migrated.status, 200);
    let migrated = migrated.json()?;
    assert_eq!(migrated["document"]["id"], "game-f1-24");
    assert_eq!(migrated["document"]["name"], "My v0.1.0 Layout");
    assert_eq!(migrated["document"]["revision"], 1);
    assert_eq!(migrated["document"]["theme"]["accent"], "#123456");

    host.running.shutdown().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_untouched_v0_1_0_named_default_is_also_migrated() -> Result<(), Box<dyn Error>> {
    let host = TestHost::start().await?;
    let legacy_id = LayoutId::new("default")?;
    let legacy = LayoutDocument::empty(legacy_id, "F1 24 Default")?;
    host.repository.save(&legacy, 0)?;
    let session = host.session().await?;
    let authorization = format!("Bearer {session}");

    let migrated = request(
        host.running.http_address(),
        "GET",
        "/api/v1/layouts/game-ats",
        &[("Authorization", authorization.as_str())],
        &[],
    )
    .await?;
    assert_eq!(migrated.status, 200);
    let migrated = migrated.json()?;
    let document = &migrated["document"];
    assert_eq!(document["id"], "game-ats");
    assert_eq!(document["name"], "F1 24 Default");
    assert_eq!(document["revision"], 1);
    assert_eq!(document["widgets"].as_array().map(Vec::len), Some(0));

    host.running.shutdown().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn authenticated_layouts_save_atomically_and_report_revision_conflicts()
-> Result<(), Box<dyn Error>> {
    let host = TestHost::start().await?;
    let address = host.running.http_address();
    let session = host.session().await?;

    let unauthorized = request(address, "GET", "/api/v1/layouts/default", &[], &[]).await?;
    assert_eq!(unauthorized.status, 401);

    let authorization = format!("Bearer {session}");
    let initial = request(
        address,
        "GET",
        "/api/v1/layouts/default",
        &[("Authorization", authorization.as_str())],
        &[],
    )
    .await?;
    assert_eq!(initial.status, 200);
    let initial_json = initial.json()?;
    assert_eq!(initial_json["document"]["revision"], 1);
    assert_eq!(
        initial_json["document"]["widgets"].as_array().map(Vec::len),
        Some(6)
    );

    let mut changed = initial_json["document"].clone();
    changed["name"] = json!("My Track Layout");
    let changed_bytes = serde_json::to_vec(&changed)?;
    let saved = request(
        address,
        "PUT",
        "/api/v1/layouts/default",
        &[
            ("Authorization", authorization.as_str()),
            ("Content-Type", "application/json"),
        ],
        &changed_bytes,
    )
    .await?;
    assert_eq!(saved.status, 200);
    assert_eq!(saved.json()?["document"]["revision"], 2);

    let mut stale = initial_json["document"].clone();
    stale["name"] = json!("Stale Client");
    let stale_bytes = serde_json::to_vec(&stale)?;
    let conflict = request(
        address,
        "PUT",
        "/api/v1/layouts/default",
        &[
            ("Authorization", authorization.as_str()),
            ("Content-Type", "application/json"),
        ],
        &stale_bytes,
    )
    .await?;
    assert_eq!(conflict.status, 409);
    let conflict_json = conflict.json()?;
    assert_eq!(conflict_json["code"], "revision_conflict");
    assert_eq!(conflict_json["current"]["document"]["revision"], 2);
    assert_eq!(
        conflict_json["current"]["document"]["name"],
        "My Track Layout"
    );

    host.running.shutdown().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn layout_api_rejects_unknown_code_settings_future_schemas_and_oversized_bodies()
-> Result<(), Box<dyn Error>> {
    let host = TestHost::start().await?;
    let address = host.running.http_address();
    let session = host.session().await?;
    let authorization = format!("Bearer {session}");
    let initial = request(
        address,
        "GET",
        "/api/v1/layouts/default",
        &[("Authorization", authorization.as_str())],
        &[],
    )
    .await?
    .json()?["document"]
        .clone();

    for invalid in [
        {
            let mut value = initial.clone();
            value["widgets"][0]["componentType"] = json!("custom.user-script");
            value
        },
        {
            let mut value = initial.clone();
            value["widgets"][0]["settings"] = json!({"onData": "alert(1)"});
            value
        },
        {
            let mut value = initial.clone();
            value["schemaVersion"] = json!(99);
            value
        },
    ] {
        let body = serde_json::to_vec(&invalid)?;
        let response = request(
            address,
            "PUT",
            "/api/v1/layouts/default",
            &[
                ("Authorization", authorization.as_str()),
                ("Content-Type", "application/json"),
            ],
            &body,
        )
        .await?;
        assert_eq!(response.status, 422, "response body: {:?}", response.body);
    }

    let oversized = vec![b' '; MAX_LAYOUT_BYTES + 1];
    let response = request(
        address,
        "PUT",
        "/api/v1/layouts/default",
        &[
            ("Authorization", authorization.as_str()),
            ("Content-Type", "application/json"),
        ],
        &oversized,
    )
    .await?;
    assert_eq!(response.status, 413);

    host.running.shutdown().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn layout_get_recovers_the_last_known_good_revision() -> Result<(), Box<dyn Error>> {
    let host = TestHost::start().await?;
    let address = host.running.http_address();
    let session = host.session().await?;
    let authorization = format!("Bearer {session}");
    let initial = request(
        address,
        "GET",
        "/api/v1/layouts/default",
        &[("Authorization", authorization.as_str())],
        &[],
    )
    .await?;
    assert_eq!(initial.status, 200);

    let id = LayoutId::new("default")?;
    fs::write(host.repository.layout_path(&id), b"{ broken layout")?;
    let recovered = request(
        address,
        "GET",
        "/api/v1/layouts/default",
        &[("Authorization", authorization.as_str())],
        &[],
    )
    .await?;
    assert_eq!(recovered.status, 200);
    let recovered_json = recovered.json()?;
    assert_eq!(recovered_json["recovered"], true);
    assert_eq!(recovered_json["document"]["revision"], 1);

    host.running.shutdown().await?;
    Ok(())
}

async fn request(
    address: std::net::SocketAddr,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) -> Result<HttpResponse, Box<dyn Error>> {
    let mut stream = TcpStream::connect(address).await?;
    let mut head = format!(
        "{method} {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\nContent-Length: {}\r\n",
        body.len()
    );
    for (name, value) in headers {
        head.push_str(name);
        head.push_str(": ");
        head.push_str(value);
        head.push_str("\r\n");
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes()).await?;
    stream.write_all(body).await?;
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).await?;
    let separator = bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| io::Error::other("HTTP response has no body separator"))?;
    let response_head = std::str::from_utf8(&bytes[..separator])?;
    let status = response_head
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| io::Error::other("HTTP response has no status"))?
        .parse()?;
    Ok(HttpResponse {
        status,
        body: bytes[separator + 4..].to_vec(),
    })
}
