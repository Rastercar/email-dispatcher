use lapin::message::Delivery;

use crate::controller::router::nack_delivery;

pub async fn handle_delivery_without_corresponding_rpc(delivery: Delivery) -> Result<(), String> {
    // TODO: log to jaeger that a RPC was not found ?
    nack_delivery(&delivery).await
}
