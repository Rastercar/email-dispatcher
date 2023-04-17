use crate::controller::nack_delivery;
use lapin::message::Delivery;

pub async fn handle_delivery_without_corresponding_rpc(delivery: Delivery) -> Result<(), String> {
    // TODO: log to jaeger that a RPC was not found ?
    nack_delivery(&delivery).await
}
