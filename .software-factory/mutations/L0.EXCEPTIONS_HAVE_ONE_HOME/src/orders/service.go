package orders

// Defined in a service instead of the domain's errors module.
type OrderRejectedError struct {
	Reason string
}
