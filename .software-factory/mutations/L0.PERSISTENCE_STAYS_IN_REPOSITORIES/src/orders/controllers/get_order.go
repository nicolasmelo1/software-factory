package controllers

func GetOrder(orderID string) (*Order, error) {
	return db.Query("select * from orders where id = $1", orderID)
}
