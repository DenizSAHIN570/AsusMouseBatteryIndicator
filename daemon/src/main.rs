mod dbus;
mod hid;
mod notification;
mod notifier;
mod predictor;

use anyhow::Result;
use dbus::device::{BatteryDevice, BatteryState};
use hid::{asus::AsusDevice, asus::ASUS_KNOWN_IDS, BatteryStatus, MouseDevice};
use notifier::NotificationPolicy;
use predictor::BatteryPredictor;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::{interval, MissedTickBehavior};
use tracing_subscriber::EnvFilter;
use zbus::Connection;
use zvariant::OwnedObjectPath;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("mouse_battery=info".parse()?))
        .init();

    tracing::info!("mouse-battery daemon starting");

    let state = Arc::new(Mutex::new(BatteryState {
        device_name: "Unknown".to_string(),
        status: "unknown".to_string(),
        is_present: false,
        ..Default::default()
    }));

    let conn = dbus::build_connection(Arc::clone(&state)).await?;
    tracing::info!("DBus service registered as {}", dbus::SERVICE_NAME);

    let poll_secs: u64 = std::env::var("MOUSE_BATTERY_INTERVAL")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);

    tokio::select! {
        _ = run_poll_loop(conn, state, Duration::from_secs(poll_secs)) => {},
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Shutting down");
        }
    }

    Ok(())
}

async fn run_poll_loop(
    conn: Connection,
    state: Arc<Mutex<BatteryState>>,
    poll_interval: Duration,
) {
    let mut ticker = interval(poll_interval);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

    let mut predictor = BatteryPredictor::new();
    let mut notify_policy = NotificationPolicy::new();
    let mut prev_is_present = false;

    loop {
        ticker.tick().await;

        let reading = match try_query_device().await {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("Device query failed: {e:#}");
                if prev_is_present {
                    set_not_present(&conn, &state).await;
                    emit_device_removed(&conn).await;
                    prev_is_present = false;
                }
                notify_policy.reset();
                continue;
            }
        };

        // Cross-check: if the cable is connected but voltage is consistently
        // falling, the cable isn't delivering enough current. Override status
        // to Discharging so the user knows something is wrong.
        // On the first reading (no history yet) we trust byte[9] optimistically.
        let effective_status = if reading.cable_connected {
            match predictor.cable_is_charging() {
                Some(false) => {
                    tracing::warn!(
                        "Cable connected but voltage dropping — weak/faulty cable ({}mV)",
                        reading.voltage_mv
                    );
                    BatteryStatus::Discharging
                }
                _ => reading.status.clone(), // None = not enough data yet, Some(true) = ok
            }
        } else {
            reading.status.clone()
        };

        predictor.push(reading.percentage, reading.voltage_mv, &effective_status);

        let time_to_empty = if effective_status == BatteryStatus::Discharging {
            predictor.time_to_empty()
        } else {
            0
        };
        let time_to_full = if effective_status == BatteryStatus::Charging {
            predictor.time_to_full()
        } else {
            0
        };

        {
            let mut s = state.lock().unwrap();
            s.percentage = reading.percentage;
            s.status = effective_status.as_str().to_string();
            s.time_to_empty = time_to_empty;
            s.time_to_full = time_to_full;
            s.voltage_mv = reading.voltage_mv as u32;
            s.is_present = true;
        }

        if !prev_is_present {
            update_device_name(&state).await;
            emit_device_added(&conn).await;
            prev_is_present = true;
        }

        publish_battery_update(&conn, reading.percentage, effective_status.as_str()).await;

        // Notifications are driven by percentage alone — the firmware's status
        // byte is unreliable, and gating on it made FullyCharged unreachable
        // (the cable forces status to Charging, but the battery can only fill
        // while the cable is connected).
        let fire = notify_policy.update(reading.percentage);

        if fire.low {
            tracing::info!("Battery low: {}%", reading.percentage);
            if let Err(e) = notification::send_low_battery(&conn, reading.percentage).await {
                tracing::warn!("Failed to send low-battery notification: {e:#}");
            }
            emit_battery_low(&conn, reading.percentage).await;
        }

        if fire.full {
            tracing::info!("Battery fully charged");
            if let Err(e) = notification::send_battery_full(&conn).await {
                tracing::warn!("Failed to send full-battery notification: {e:#}");
            }
            emit_battery_full(&conn).await;
        }
    }
}

/// Find candidates, try each in order, return the first successful reading.
/// HidDevice is !Send so everything runs inside spawn_blocking.
async fn try_query_device() -> Result<hid::BatteryReading> {
    tokio::task::spawn_blocking(|| {
        let candidates = hid::find_hidraw_nodes(ASUS_KNOWN_IDS)?;
        if candidates.is_empty() {
            anyhow::bail!("No supported mouse found");
        }
        let mut last_err = anyhow::anyhow!("No candidates responded");
        for m in candidates {
            match AsusDevice::open(&m.dev_node).and_then(|d| d.query_battery()) {
                Ok(reading) => return Ok(reading),
                Err(e) => {
                    tracing::debug!("Candidate {} failed: {e:#}", m.dev_node);
                    last_err = e;
                }
            }
        }
        Err(last_err)
    })
    .await?
}

async fn publish_battery_update(conn: &Connection, percentage: u8, status: &str) {
    let obj_server = conn.object_server();
    let Ok(iface_ref) = obj_server
        .interface::<_, BatteryDevice>(dbus::DEVICE0_PATH)
        .await
    else {
        return;
    };

    let emitter = iface_ref.signal_emitter();

    // Notify DBus clients of changed properties
    {
        let iface = iface_ref.get().await;
        let _ = iface.device_name_changed(&emitter).await;
        let _ = iface.percentage_changed(&emitter).await;
        let _ = iface.status_changed(&emitter).await;
        let _ = iface.time_to_empty_changed(&emitter).await;
        let _ = iface.time_to_full_changed(&emitter).await;
        let _ = iface.voltage_mv_changed(&emitter).await;
        let _ = iface.is_present_changed(&emitter).await;
    }

    // Emit BatteryChanged signal
    let _ = BatteryDevice::battery_changed(&emitter, percentage, status.to_string()).await;
}

async fn emit_battery_low(conn: &Connection, percentage: u8) {
    let obj_server = conn.object_server();
    if let Ok(iface_ref) = obj_server
        .interface::<_, BatteryDevice>(dbus::DEVICE0_PATH)
        .await
    {
        let _ = BatteryDevice::battery_low(iface_ref.signal_emitter(), percentage).await;
    }
}

async fn emit_battery_full(conn: &Connection) {
    let obj_server = conn.object_server();
    if let Ok(iface_ref) = obj_server
        .interface::<_, BatteryDevice>(dbus::DEVICE0_PATH)
        .await
    {
        let _ = BatteryDevice::battery_full(iface_ref.signal_emitter()).await;
    }
}

async fn set_not_present(conn: &Connection, state: &Arc<Mutex<BatteryState>>) {
    {
        let mut s = state.lock().unwrap();
        s.is_present = false;
        s.status = "unknown".to_string();
        s.time_to_empty = 0;
        s.time_to_full = 0;
    }
    let obj_server = conn.object_server();
    if let Ok(iface_ref) = obj_server
        .interface::<_, BatteryDevice>(dbus::DEVICE0_PATH)
        .await
    {
        let emitter = iface_ref.signal_emitter();
        let iface = iface_ref.get().await;
        let _ = iface.is_present_changed(&emitter).await;
        let _ = iface.status_changed(&emitter).await;
    }
}

async fn update_device_name(state: &Arc<Mutex<BatteryState>>) {
    let mut s = state.lock().unwrap();
    if s.device_name == "Unknown" {
        s.device_name = "TUF GAMING MINI WL MOUSE MIKU".to_string();
    }
}

async fn emit_device_added(conn: &Connection) {
    use dbus::manager::BatteryManager;
    let obj_server = conn.object_server();
    if let Ok(iface_ref) = obj_server
        .interface::<_, BatteryManager>(dbus::MANAGER_PATH)
        .await
    {
        let emitter = iface_ref.signal_emitter();
        let path = OwnedObjectPath::try_from(dbus::DEVICE0_PATH).unwrap();
        let _ = BatteryManager::device_added(&emitter, path).await;
    }
}

async fn emit_device_removed(conn: &Connection) {
    use dbus::manager::BatteryManager;
    let obj_server = conn.object_server();
    if let Ok(iface_ref) = obj_server
        .interface::<_, BatteryManager>(dbus::MANAGER_PATH)
        .await
    {
        let emitter = iface_ref.signal_emitter();
        let path = OwnedObjectPath::try_from(dbus::DEVICE0_PATH).unwrap();
        let _ = BatteryManager::device_removed(&emitter, path).await;
    }
}
