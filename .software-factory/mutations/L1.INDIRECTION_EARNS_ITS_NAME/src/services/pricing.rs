use crate::db::price as price_row;

pub fn price(order: &Order) -> Money {
    price_row(order)
}
