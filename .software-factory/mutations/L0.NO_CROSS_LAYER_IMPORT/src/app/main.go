package app

import "example.com/billing/internal/rates"

func Price(o Order) int {
	return rates.Compute(o)
}
