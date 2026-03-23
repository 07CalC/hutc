local client = http()
client:base_url("https://crux-pied.vercel.app")

test("GET /explore status test", function()
  local res = client:req():path("/explore"):get()

  expect(res.status):to_equal(200)
end)
