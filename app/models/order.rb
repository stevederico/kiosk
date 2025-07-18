class Order < ApplicationRecord
    serialize :items, Array
end
