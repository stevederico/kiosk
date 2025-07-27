class PagesController < ApplicationController
  def index
    render file: Rails.root.join('public', 'kiosk.html'), layout: false
  end
end