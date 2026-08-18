def get_order(order_id, db):
    return db.execute("select * from orders where id = %s", order_id)
