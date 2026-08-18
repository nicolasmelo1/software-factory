package handlers

func Register(r *gin.Engine) {
	r.GET("/orders", listOrders)
	r.POST("/orders", createOrder)
}
