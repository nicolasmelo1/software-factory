use billing::internal::rates::compute;

pub fn price(order: &Order) -> u64 {
    compute(order)
}
