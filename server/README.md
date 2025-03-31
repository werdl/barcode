# server
> contains web server portion of project
## test by curl
### Add a new item
curl -X POST http://127.0.0.1:3000/new \
-H "Content-Type: application/json" \
-d '{"name": "item1", "barcode": 42, "location": "location1"}'

### Get all items
curl -X GET http://127.0.0.1:3000/all

### Get a specific item by barcode
curl -X GET http://127.0.0.1:3000/item/42

### Modify an item
curl -X POST http://127.0.0.1:3000/modify \
-H "Content-Type: application/json" \
-d '{"name": "updated_item1", "barcode": 42, "location": "new_location"}'

### Delete an item
curl -X DELETE http://127.0.0.1:3000/delete/42

### Log an item (update its last_seen timestamp)
curl -X POST http://127.0.0.1:3000/log/43
