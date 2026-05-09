pub mod device;
pub mod manager;

use anyhow::Result;
use std::sync::{Arc, Mutex};
use zbus::{connection, Connection};
use zvariant::OwnedObjectPath;

use device::{BatteryDevice, BatteryState};
use manager::BatteryManager;

pub const SERVICE_NAME: &str = "com.mousewatch.Battery";
pub const MANAGER_PATH: &str = "/com/mousewatch/Battery";
pub const DEVICE0_PATH: &str = "/com/mousewatch/Battery/device0";

pub async fn build_connection(state: Arc<Mutex<BatteryState>>) -> Result<Connection> {
    let device0_path = OwnedObjectPath::try_from(DEVICE0_PATH)
        .expect("DEVICE0_PATH is a valid object path");

    let conn = connection::Builder::session()?
        .name(SERVICE_NAME)?
        .serve_at(DEVICE0_PATH, BatteryDevice { state })?
        .serve_at(
            MANAGER_PATH,
            BatteryManager {
                device_paths: vec![device0_path],
            },
        )?
        .build()
        .await?;

    Ok(conn)
}
