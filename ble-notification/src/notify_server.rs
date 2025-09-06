use defmt::Format;
use nrf_softdevice::ble::{
    Connection, DeferredReadReply, DeferredWriteReply,
    gatt_server::{Server, WriteOp},
};

pub struct NotifyServer {
    pub cccd_handle: u16,
    #[allow(unused)]
    pub value_handle: u16,
}

impl NotifyServer {
    pub fn new(cccd_handle: u16, value_handle: u16) -> Self {
        Self {
            cccd_handle,
            value_handle,
        }
    }
}

pub enum NotifyEvent {
    NotifyEnabled,
}

impl Format for NotifyEvent {
    fn format(&self, _fmt: defmt::Formatter) {}
}

impl Server for NotifyServer {
    type Event = NotifyEvent;

    fn on_write(
        &self,
        _conn: &Connection,
        handle: u16,
        _op: WriteOp,
        _offset: usize,
        data: &[u8],
    ) -> Option<Self::Event> {
        if handle == self.cccd_handle && data == [0x01, 0x00] {
            return Some(NotifyEvent::NotifyEnabled);
        }

        None
    }

    fn on_deferred_read(
        &self,
        handle: u16,
        offset: usize,
        reply: DeferredReadReply,
    ) -> Option<Self::Event> {
        let _ = (handle, offset, reply);
        None
    }

    fn on_deferred_write(
        &self,
        handle: u16,
        op: WriteOp,
        offset: usize,
        data: &[u8],
        reply: DeferredWriteReply,
    ) -> Option<Self::Event> {
        let _ = (handle, op, offset, data, reply);
        None
    }

    fn on_notify_tx_complete(&self, conn: &Connection, count: u8) -> Option<Self::Event> {
        let _ = (conn, count);
        None
    }

    fn on_indicate_confirm(&self, conn: &Connection, handle: u16) -> Option<Self::Event> {
        let _ = (conn, handle);
        None
    }

    fn on_services_changed_confirm(&self, conn: &Connection) -> Option<Self::Event> {
        let _ = conn;
        None
    }

    fn on_timeout(&self, conn: &Connection) -> Option<Self::Event> {
        let _ = conn;
        None
    }
}
