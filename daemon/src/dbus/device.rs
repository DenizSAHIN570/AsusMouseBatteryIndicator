use std::sync::{Arc, Mutex};
use zbus::interface;
use zbus::object_server::SignalEmitter;

#[derive(Debug, Clone, Default)]
pub struct BatteryState {
    pub device_name: String,
    pub percentage: u8,
    pub status: String,
    pub time_to_full: u32,
    pub time_to_empty: u32,
    pub voltage_mv: u32,
    pub is_present: bool,
}

pub struct BatteryDevice {
    pub state: Arc<Mutex<BatteryState>>,
}

#[interface(name = "com.mousewatch.Battery1")]
impl BatteryDevice {
    #[zbus(property)]
    async fn device_name(&self) -> String {
        self.state.lock().unwrap().device_name.clone()
    }

    #[zbus(property)]
    async fn percentage(&self) -> u8 {
        self.state.lock().unwrap().percentage
    }

    #[zbus(property)]
    async fn status(&self) -> String {
        self.state.lock().unwrap().status.clone()
    }

    #[zbus(property)]
    async fn time_to_full(&self) -> u32 {
        self.state.lock().unwrap().time_to_full
    }

    #[zbus(property)]
    async fn time_to_empty(&self) -> u32 {
        self.state.lock().unwrap().time_to_empty
    }

    #[zbus(property)]
    async fn voltage_mv(&self) -> u32 {
        self.state.lock().unwrap().voltage_mv
    }

    #[zbus(property)]
    async fn is_present(&self) -> bool {
        self.state.lock().unwrap().is_present
    }

    #[zbus(property)]
    async fn daemon_version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    #[zbus(signal)]
    pub async fn battery_changed(
        emitter: &SignalEmitter<'_>,
        percentage: u8,
        status: String,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn battery_low(
        emitter: &SignalEmitter<'_>,
        percentage: u8,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn battery_full(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}
