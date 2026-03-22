local client = http()
client:base_url("https://crux.hs.vc")

test("GET /", function()
	local res = client:get("/")
	expect(res.status):to_equal(200)
end)

test("GET /explore", function()
	local res = client:get("/explore")
	expect(res.status):to_equal(200)
end)
