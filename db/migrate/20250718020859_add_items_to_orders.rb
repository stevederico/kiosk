class AddItemsToOrders < ActiveRecord::Migration[8.0]
  def change
    add_column :orders, :items, :text
  end
end
