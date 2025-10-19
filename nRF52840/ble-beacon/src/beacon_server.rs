use defmt::debug;
use nrf_softdevice::ble::gatt_server::Server;

pub struct BeaconEvent {}

pub struct BeaconServer {
    // Beacon server implementation details
}

impl Server for BeaconServer {
    type Event = BeaconEvent;

    fn on_write(
        &self,
        _conn: &nrf_softdevice::ble::Connection,
        _handle: u16,
        _op: nrf_softdevice::ble::gatt_server::WriteOp,
        _offset: usize,
        data: &[u8],
    ) -> Option<Self::Event> {
        debug!("on_write: {}", data);
        Some(BeaconEvent {})
    }
}
