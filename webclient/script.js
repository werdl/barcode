const SERVER = "127.0.0.1:3000";

// get all items
function getAllItems() {
    fetch(`http://${SERVER}/all`)
        .then(response => response.json())
        .then(data => {
            console.log('All items:', data);
            data;
        })
        .catch(error => console.error('Error fetching all items:', error));
}

// add a new item
function addItem(name, barcode, location) {
    const item = { name, barcode, location };
    fetch(`http://${SERVER}/new`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(item)
    })
        .then(response => response.json())
        .then(data => console.log('Added item:', data))
        .catch(error => console.error('Error adding item:', error));
}

// modify an item
function modifyItem(name, barcode, location) {
    const item = { name, barcode, location };
    fetch(`http://${SERVER}/modify`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(item)
    })
        .then(response => response.json())
        .then(data => console.log('Modified item:', data))
        .catch(error => console.error('Error modifying item:', error));
}

// delete an item
function deleteItem(barcode) {
    fetch(`http://${SERVER}/delete/${barcode}`)
        .then(response => response.json())
        .then(data => console.log('Deleted item:', data))
        .catch(error => console.error('Error deleting item:', error));
}

// log an item (update its last_seen timestamp)
function logItem(barcode) {
    fetch(`http://${SERVER}/log/${barcode}`, {
        method: 'POST'
    })
        .then(response => response.json())
        .then(data => console.log('Logged item:', data))
        .catch(error => console.error('Error logging item:', error));
}

// get a specific item by barcode
function getItem(barcode) {
    fetch(`http://${SERVER}/item/${barcode}`)
        .then(response => response.json())
        .then(data => {
            console.log('Item:', data);
            data;
        })
        .catch(error => console.error('Error fetching item:', error));
}

// get all items and add to the DOM
function getAllItemsDOM() {
    fetch(`http://${SERVER}/all`)
        .then(response => response.json())
        .then(data => {
            const table = document.getElementById('table');
            table.innerHTML = ''; // Clear existing items

            // Create table headers
            const headerRow = document.createElement('tr');
            ['Name', 'Barcode', 'Location'].forEach(headerText => {
                const th = document.createElement('th');
                th.textContent = headerText;
                headerRow.appendChild(th);
            });
            table.appendChild(headerRow);

            // Populate table rows
            data.forEach(item => {
                const row = document.createElement('tr');
                row.id = item.barcode; // Set the row ID to the barcode for easy access
                row.onclick = function () {
                    // pop up with buttons: modify, delete, log
                    const popup = document.createElement('div');
                    popup.className = 'popup';
                    popup.innerHTML = `
                        <h2>${item.name}</h2>
                        <button onclick="modifyItem('${item.name}', '${item.barcode}', '${item.location}');getAllItemsDOM()">Modify</button>
                        <button onclick="deleteItem('${item.barcode}');getAllItemsDOM()">Delete</button>
                        <button onclick="logItem('${item.barcode}');getAllItemsDOM()">Log</button>
                        <button onclick="closePopup('${item.barcode}');getAllItemsDOM()">Close</button>
                    `;
                    document.body.appendChild(popup);
                    popup.style.display = 'block';
                }
                row.onmouseover = function () {
                    this.style.backgroundColor = '#f0f0f0';
                }
                row.onmouseout = function () {
                    this.style.backgroundColor = '';
                }

                const nameCell = document.createElement('td');
                nameCell.textContent = item.name;
                row.appendChild(nameCell);

                const barcodeCell = document.createElement('td');
                barcodeCell.textContent = item.barcode;
                row.appendChild(barcodeCell);

                const locationCell = document.createElement('td');
                locationCell.textContent = item.location;
                row.appendChild(locationCell);

                const lastSeenCell = document.createElement('td');
                lastSeenCell.textContent = item.last_seen ? new Date(item.last_seen * 1000).toLocaleString() : 'Never';
                row.appendChild(lastSeenCell);

                table.appendChild(row);
            });
        })
        .catch(error => console.error('Error fetching all items:', error));
}

function closePopup(barcode) {
    const popup = document.querySelector(`.popup`);
    if (popup) {
        popup.style.display = 'none';
        document.body.removeChild(popup);
    }
}
