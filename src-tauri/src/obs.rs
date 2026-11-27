use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    net::{Ipv4Addr, SocketAddrV4, TcpStream},
    time::Duration,
};
use tungstenite::{client, Message, WebSocket};

const OBS_READ_TIMEOUT: Duration = Duration::from_secs(4);
const MAX_PASSWORD_BYTES: usize = 4096;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObsStatus {
    pub port: u16,
    pub obs_version: String,
    pub websocket_version: String,
    pub current_scene: String,
    pub scenes: Vec<ObsScene>,
    pub authenticated: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObsScene {
    pub scene_name: String,
    pub scene_index: i32,
}

#[derive(Deserialize)]
struct ProtocolMessage {
    op: u8,
    d: Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HelloData {
    authentication: Option<AuthenticationChallenge>,
}

#[derive(Deserialize)]
struct AuthenticationChallenge {
    challenge: String,
    salt: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SceneList {
    current_program_scene_name: String,
    scenes: Vec<ObsScene>,
}

pub fn read_status(port: u16, password: Option<&str>) -> Result<ObsStatus, String> {
    if port == 0 {
        return Err("Enter a valid OBS WebSocket port.".to_string());
    }
    if password.is_some_and(|value| value.len() > MAX_PASSWORD_BYTES) {
        return Err("The OBS WebSocket password is too long.".to_string());
    }

    let address = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port);
    let stream = TcpStream::connect_timeout(&address.into(), OBS_READ_TIMEOUT)
        .map_err(|error| format!("Could not connect to OBS at 127.0.0.1:{port}: {error}"))?;
    stream
        .set_read_timeout(Some(OBS_READ_TIMEOUT))
        .map_err(|error| format!("Could not set the OBS connection timeout: {error}"))?;
    stream
        .set_write_timeout(Some(OBS_READ_TIMEOUT))
        .map_err(|error| format!("Could not set the OBS connection timeout: {error}"))?;

    let (mut socket, _) = client(format!("ws://127.0.0.1:{port}"), stream)
        .map_err(|error| format!("Could not complete the OBS WebSocket handshake: {error}"))?;
    let hello_message = read_protocol_message(&mut socket)?;
    if hello_message.op != 0 {
        return Err(
            "OBS sent an unexpected message instead of its WebSocket greeting.".to_string(),
        );
    }
    let hello: HelloData = serde_json::from_value(hello_message.d)
        .map_err(|error| format!("Could not parse the OBS WebSocket greeting: {error}"))?;
    let authentication_required = hello.authentication.is_some();
    let authentication = match (hello.authentication, password) {
        (Some(challenge), Some(password)) if !password.is_empty() => Some(authentication_response(
            password,
            &challenge.salt,
            &challenge.challenge,
        )),
        (Some(_), _) => {
            return Err("OBS requires a WebSocket password. Enter it and reconnect.".to_string());
        }
        (None, _) => None,
    };

    let mut identify = json!({
        "op": 1,
        "d": {
            "rpcVersion": 1,
            "eventSubscriptions": 0
        }
    });
    if let Some(authentication) = authentication {
        identify["d"]["authentication"] = json!(authentication);
    }
    send_protocol_message(&mut socket, &identify)?;
    let identified = read_protocol_message(&mut socket)?;
    if identified.op != 2 {
        let detail = identified
            .d
            .get("comment")
            .and_then(Value::as_str)
            .unwrap_or("OBS did not accept the WebSocket connection.");
        return Err(detail.to_string());
    }

    let version = request(&mut socket, "GetVersion", "wsc-version")?;
    let scenes = request(&mut socket, "GetSceneList", "wsc-scenes")?;
    let obs_version = value_string(&version, "obsVersion")?;
    let websocket_version = value_string(&version, "obsWebSocketVersion")?;
    let scene_list: SceneList = serde_json::from_value(scenes)
        .map_err(|error| format!("Could not parse the OBS scene list: {error}"))?;
    if scene_list.current_program_scene_name.is_empty() {
        return Err("OBS did not report a current scene.".to_string());
    }

    let _ = socket.close(None);
    Ok(ObsStatus {
        port,
        obs_version,
        websocket_version,
        current_scene: scene_list.current_program_scene_name,
        scenes: scene_list.scenes,
        authenticated: authentication_required,
    })
}

fn authentication_response(password: &str, salt: &str, challenge: &str) -> String {
    let secret = STANDARD.encode(Sha256::digest(format!("{password}{salt}").as_bytes()));
    STANDARD.encode(Sha256::digest(format!("{secret}{challenge}").as_bytes()))
}

fn send_protocol_message(socket: &mut WebSocket<TcpStream>, message: &Value) -> Result<(), String> {
    socket
        .send(Message::Text(message.to_string().into()))
        .map_err(|error| format!("Could not send an OBS WebSocket request: {error}"))
}

fn read_protocol_message(socket: &mut WebSocket<TcpStream>) -> Result<ProtocolMessage, String> {
    loop {
        match socket
            .read()
            .map_err(|error| format!("Could not read from OBS WebSocket: {error}"))?
        {
            Message::Text(text) => {
                return serde_json::from_str(text.as_ref())
                    .map_err(|error| format!("OBS sent invalid WebSocket JSON: {error}"));
            }
            Message::Ping(payload) => socket
                .send(Message::Pong(payload))
                .map_err(|error| format!("Could not respond to OBS WebSocket ping: {error}"))?,
            Message::Pong(_) => {}
            Message::Close(_) => {
                return Err("OBS closed the WebSocket connection unexpectedly.".to_string());
            }
            Message::Binary(_) => {
                return Err("OBS sent an unexpected binary WebSocket message.".to_string());
            }
            Message::Frame(_) => {}
        }
    }
}

fn request(
    socket: &mut WebSocket<TcpStream>,
    request_type: &str,
    request_id: &str,
) -> Result<Value, String> {
    send_protocol_message(
        socket,
        &json!({
            "op": 6,
            "d": {
                "requestType": request_type,
                "requestId": request_id
            }
        }),
    )?;
    let response = read_protocol_message(socket)?;
    if response.op != 7 {
        return Err(format!(
            "OBS returned an unexpected response to {request_type}."
        ));
    }
    if response.d.get("requestId").and_then(Value::as_str) != Some(request_id) {
        return Err(format!(
            "OBS returned a mismatched response for {request_type}."
        ));
    }
    let status = response
        .d
        .get("requestStatus")
        .ok_or_else(|| format!("OBS omitted the status for {request_type}."))?;
    if status.get("result").and_then(Value::as_bool) != Some(true) {
        let code = status
            .get("code")
            .and_then(Value::as_i64)
            .map(|code| format!(" ({code})"))
            .unwrap_or_default();
        let comment = status
            .get("comment")
            .and_then(Value::as_str)
            .unwrap_or("Request failed.");
        return Err(format!(
            "OBS {request_type} request failed{code}: {comment}"
        ));
    }
    response
        .d
        .get("responseData")
        .cloned()
        .ok_or_else(|| format!("OBS omitted data for {request_type}."))
}

fn value_string(value: &Value, key: &str) -> Result<String, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("OBS did not report '{key}'."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn obs_authentication_response_matches_the_challenge_algorithm() {
        let response = authentication_response("password", "salt", "challenge");
        assert_eq!(response, "zTM5ki6L2vVvBQiTG9ckH1Lh64AbnCf6XZ226UmnkIA=");
    }

    #[test]
    fn obs_password_limit_and_port_are_validated_before_connecting() {
        assert!(read_status(0, None).unwrap_err().contains("valid OBS"));
        assert!(read_status(4455, Some(&"x".repeat(MAX_PASSWORD_BYTES + 1)))
            .unwrap_err()
            .contains("too long"));
    }

    #[test]
    fn obs_scene_response_uses_the_websocket_api_field_names() {
        let response: SceneList = serde_json::from_value(json!({
            "currentProgramSceneName": "Gameplay",
            "scenes": [
                {"sceneName": "Gameplay", "sceneIndex": 0},
                {"sceneName": "BRB", "sceneIndex": 1}
            ]
        }))
        .unwrap();
        assert_eq!(response.current_program_scene_name, "Gameplay");
        assert_eq!(response.scenes[1].scene_name, "BRB");
    }
}
