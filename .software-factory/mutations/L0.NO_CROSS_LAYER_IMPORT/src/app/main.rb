require_relative "../billing/_internal/rates"

def price(order)
  compute(order)
end
