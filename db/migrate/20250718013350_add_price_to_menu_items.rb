class AddPriceToMenuItems < ActiveRecord::Migration[8.0]
  def change
    add_column :menu_items, :price, :decimal
  end
end
