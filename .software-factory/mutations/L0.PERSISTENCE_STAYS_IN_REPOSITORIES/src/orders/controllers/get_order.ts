export async function getOrder(orderId: string) {
  return db.query("select * from orders where id = $1", [orderId]);
}
