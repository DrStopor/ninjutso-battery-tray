use gpui::{AssetSource, SharedString};
use std::borrow::Cow;

pub(super) struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        let bytes: &'static [u8] = match path {
            "icons/close.svg" => include_bytes!("../../assets/ui/close.svg"),
            "icons/mouse.svg" => include_bytes!("../../assets/ui/mouse.svg"),
            "icons/bolt.svg" => include_bytes!("../../assets/ui/bolt.svg"),
            "icons/receiver.svg" => include_bytes!("../../assets/ui/receiver.svg"),
            "icons/usb.svg" => include_bytes!("../../assets/ui/usb.svg"),
            _ => return Ok(None),
        };
        Ok(Some(Cow::Borrowed(bytes)))
    }

    fn list(&self, path: &str) -> anyhow::Result<Vec<SharedString>> {
        Ok(if path == "icons" {
            ["close", "mouse", "bolt", "receiver", "usb"]
                .map(|name| format!("icons/{name}.svg").into())
                .to_vec()
        } else {
            vec![]
        })
    }
}
