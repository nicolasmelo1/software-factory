from billing._internal.rates import compute


def price(order):
    return compute(order)
