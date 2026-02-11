require 'net/http'
require 'json'

resp = Net::HTTP.post(
  URI('http://localhost:8080/api/login'),
  { password: 'admin' }.to_json,
  { "Content-Type" => "application/json" }
)

body = JSON.parse(resp.body)
err = body["error"]
if err
  puts "ERROR: #{err}"
  exit(1)
end

token = body["auth_token"]
puts "Token: #{token}"

resp = Net::HTTP.post(
  URI('http://localhost:8080/api/net/ifstate'),
  { interface_name: 'wlan0', link_state: 'Up' }.to_json,
  { "Authorization" => "Bearer #{token}",
    "Content-Type" => "application/json" }
)

puts "Response: #{resp.body}"
