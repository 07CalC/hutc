local client = http()

-- test("GET /", function()
-- 	local res = client:get("/")
-- 	expect(res.status):to_equal(200)
-- end)
--
-- test("GET /explore", function()
-- 	local res = client:get("/explore")
-- 	expect(res.status):to_equal(200)
-- end)

test("GET json data", function()
	local res = client:get("https://jsonplaceholder.typicode.com/posts/5")
	print(res.json.body)
end)
