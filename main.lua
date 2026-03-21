test("hi", function() end)
test("hello", function()
  expect(1):to_equal(3)
end)

test("pass", function()
  expect(2):to_exist()
end)
