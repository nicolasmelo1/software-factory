import { compute } from "../billing/_internal/rates";

export const price = (order: Order) => compute(order);
