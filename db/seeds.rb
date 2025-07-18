# This file should ensure the existence of records required to run the application in every environment (production,
# development, test). The code here should be idempotent so that it can be executed at any point in every environment.
# The data can then be loaded with the bin/rails db:seed command (or created alongside the database with db:setup).
#
# Example:
#
#   ["Action", "Comedy", "Drama", "Horror"].each do |genre_name|
#     MovieGenre.find_or_create_by!(name: genre_name)
#   end

MenuItem.create(name: "Hotdog & Soda", price: 1.50)
MenuItem.create(name: "Slice Pepperoni", price: 1.99)
MenuItem.create(name: "Slice Cheese", price: 1.99)
MenuItem.create(name: "Rotisserie Chicken Caesar Salad", price: 6.99)
MenuItem.create(name: "Whole Pepperoni", price: 9.95)
MenuItem.create(name: "Whole Cheese", price: 9.95)
MenuItem.create(name: "1/2 Pepperoni 1/2 Cheese", price: 9.95)
MenuItem.create(name: "Chicken Bake", price: 3.99)
MenuItem.create(name: "20 oz Soda with Refill", price: 0.79)
MenuItem.create(name: "Double Chocolate Chunk Cookie", price: 2.49)
MenuItem.create(name: "Hot Turkey Provolone Sandwich", price: 6.99)
MenuItem.create(name: "Strawberry Banana Smoothie", price: 2.99)
MenuItem.create(name: "Cold Brew Mocha Freeze", price: 2.99)
MenuItem.create(name: "Cold Brew Latte Freeze", price: 2.99)
MenuItem.create(name: "Vanilla Ice Cream Cup", price: 1.99)
MenuItem.create(name: "Chocolate Ice Cream Cup", price: 1.99)
MenuItem.create(name: "Chocolate Vanilla Twist Ice Cream", price: 1.99)
MenuItem.create(name: "Chocolate Sundae", price: 2.49)
MenuItem.create(name: "Strawberry Sundae", price: 2.49)




