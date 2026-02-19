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
  URI('http://localhost:8080/api/net/ifmode'),
  { interfaceName: 'wlan0', interfaceMode: { type: 'AccessPoint' } }.to_json,
  { "Authorization" => "Bearer #{token}",
    "Content-Type" => "application/json" }
)

puts "HTTP Status Code: #{resp.code}"
puts "Response: #{resp.body}"
