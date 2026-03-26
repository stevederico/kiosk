class Order < ApplicationRecord
    serialize :items, coder: JSON
end
