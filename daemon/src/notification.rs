use anyhow::Result;
use std::collections::HashMap;
use zbus::Connection;
use zbus::proxy;
use zvariant::Value;

#[proxy(
    interface = "org.freedesktop.Notifications",
    default_service = "org.freedesktop.Notifications",
    default_path = "/org/freedesktop/Notifications"
)]
trait Notifications {
    async fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: &[&str],
        hints: HashMap<&str, Value<'_>>,
        expire_timeout: i32,
    ) -> zbus::Result<u32>;
}

pub async fn send_low_battery(conn: &Connection, percentage: u8) -> Result<()> {
    let proxy = NotificationsProxy::new(conn).await?;
    proxy
        .notify(
            "mouse-battery",
            0,
            "battery-caution-symbolic",
            "Mouse Battery Low",
            &format!("{}% remaining", percentage),
            &[],
            HashMap::new(),
            7000,
        )
        .await?;
    Ok(())
}

pub async fn send_battery_full(conn: &Connection) -> Result<()> {
    let proxy = NotificationsProxy::new(conn).await?;
    proxy
        .notify(
            "mouse-battery",
            0,
            "battery-full-charged-symbolic",
            "Mouse Battery Full",
            "Your mouse is fully charged.",
            &[],
            HashMap::new(),
            5000,
        )
        .await?;
    Ok(())
}
