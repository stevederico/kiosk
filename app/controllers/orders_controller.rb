class OrdersController < ApplicationController
    def create
        order = Order.create!(total: params[:total], items: params[:items])
        render json: {message: "Order created successfully", order_id: order.id}, status: :created
    rescue => e
        render json: {error: e.message}, status: :unprocessable_entity
    end
end