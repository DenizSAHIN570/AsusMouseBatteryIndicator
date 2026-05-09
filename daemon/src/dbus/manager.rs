use zbus::interface;
use zbus::object_server::SignalEmitter;
use zvariant::OwnedObjectPath;

pub struct BatteryManager {
    pub device_paths: Vec<OwnedObjectPath>,
}

#[interface(name = "com.mousewatch.BatteryManager1")]
impl BatteryManager {
    async fn get_devices(&self) -> Vec<OwnedObjectPath> {
        self.device_paths.clone()
    }

    #[zbus(signal)]
    pub async fn device_added(
        emitter: &SignalEmitter<'_>,
        path: OwnedObjectPath,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn device_removed(
        emitter: &SignalEmitter<'_>,
        path: OwnedObjectPath,
    ) -> zbus::Result<()>;
}
