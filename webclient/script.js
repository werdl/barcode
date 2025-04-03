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
    let int_barcode = parseInt(barcode);
    if (isNaN(int_barcode)) {
        console.error('Invalid barcode:', barcode);
        return;
    }
    const item = { name, "barcode": int_barcode, location };
    console.log(JSON.stringify(item));
    fetch(`http://${SERVER}/new`, {
        method: 'POST',
        body: JSON.stringify(item)
    })
        .then(response => response.json())
        .then(data => console.log('Added item: ', item))
        .catch(error => console.error('Error adding item:', error));
}

function actualLocation(location) {
    switch (location) {
        case 'l':
            return 'Levi Fox Hall Tech Box'
        case 'r':
            return 'Rig'
        case 'd':
            return 'Drama Studio Tech Box'
        case 's':
            return 'Storage outside Levi Fox Hall Tech Box'
        case _:
            return location
    }
}

// modify an item
function modifyItem(name, barcode, location) {
    let int_barcode = parseInt(barcode);
    if (isNaN(int_barcode)) {
        console.error('Invalid barcode:', barcode);
        return;
    }
    const item = { name, "barcode": int_barcode, "location": actualLocation(location) };
    fetch(`http://${SERVER}/modify`, {
        method: 'POST',
        body: JSON.stringify(item)
    })
        .then(response => response.json())
        .then(data => console.log('Modified item:', item))
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
            ['Name', 'Barcode', 'Location', 'Last Seen'].forEach(headerText => {
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
                        <button onclick="closePopup();modifyItemUI('${item.barcode}');getAllItemsDOM()">Modify</button>
                        <button onclick="deleteItem('${item.barcode}');closePopup();getAllItemsDOM()">Delete</button>
                        <button onclick="logItem('${item.barcode}');closePopup();getAllItemsDOM()">Log</button>
                        <button onclick="closePopup();closePopup();getAllItemsDOM()">Close</button>
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

function addItemUI() {
    closePopup(); // Close any existing popup

    const popup = document.createElement('div');
    popup.className = 'popup';
    popup.innerHTML = `
        <h2>Add Item</h2>
        <label for="name">Name:</label>
        <input type="text" id="name" required>
        <label for="barcode">Barcode:</label>
        <input type="text" id="barcode" required>
        <label for="location">Location:</label>
        <input type="text" id="location" required>
        <button onclick="addItem(document.getElementById('name').value, document.getElementById('barcode').value, document.getElementById('location').value);closePopup();getAllItemsDOM()">Add</button>
        <button onclick="closePopup()">Close</button>
    `;
    document.body.appendChild(popup);
    popup.style.display = 'block';
}

function modifyItemUI(barcode) {
    const item = document.getElementById(barcode);
    console.log(item)
    if (!item) {
        console.error('Item not found:', barcode);
        return;
    }
    const name = item.cells[0].textContent;
    const location = item.cells[2].textContent;

    closePopup(); // Close any existing popup

    const popup = document.createElement('div');
    popup.className = 'popup';
    popup.innerHTML = `
        <h2>Modify Item (${barcode})</h2>
        <label for="name">Name:</label>
        <input type="text" id="name" value="${name}" required>
        <label for="location">Location:</label>
        <input type="text" id="location" value="${location}" required>
        <button onclick="modifyItem(document.getElementById('name').value, ${barcode}, document.getElementById('location').value);closePopup();getAllItemsDOM()">Modify</button>
        <button onclick="closePopup()">Close</button>
    `;
    document.body.appendChild(popup);
    popup.style.display = 'block';
}

function closePopup() {
    const popup = document.querySelector(`.popup`);
    if (popup) {
        popup.style.display = 'none';
        document.body.removeChild(popup);
    }
}



// Set a cookie
function setCookie(name, value, days) {
    const date = new Date();
    date.setTime(date.getTime() + (days * 24 * 60 * 60 * 1000));
    const expires = "expires=" + date.toUTCString();
    document.cookie = `${name}=${value};${expires};path=/`;
}

// Get a cookie
function getCookie(name) {
    const nameEQ = name + "=";
    const cookies = document.cookie.split(';');
    for (let i = 0; i < cookies.length; i++) {
        let cookie = cookies[i].trim();
        if (cookie.indexOf(nameEQ) === 0) {
            return cookie.substring(nameEQ.length, cookie.length);
        }
    }
    return null;
}

// Initialize SERVER from cookie or default value
let SERVER = getCookie('server') || "127.0.0.1";

checkForRun();

// refresh every 5s
setInterval(getAllItemsDOM, 10000);

function checkForRun() {
    const timer = setTimeout(() => {
        console.error('Server is not running');
        SERVER = prompt('Server is not running. Please enter the server address:', SERVER);
        if (SERVER) {
            setCookie('server', SERVER, 7); // Save server address in cookie for 7 days
            location.reload();
        }
    }, 3000);

    fetch(`http://${SERVER}/all`)
        .then(response => {
            if (response.ok) {
                clearTimeout(timer);
                console.log('Server is running');
                getAllItemsDOM();
            } else {
                console.error('Server is not running');
            }
        })
        .catch(error => {
            console.error('Error fetching item 42:', error);
            SERVER = prompt('Server is not running. Please enter the server address:', SERVER);
            if (SERVER) {
                setCookie('server', SERVER, 7); // Save server address in cookie for 7 days
                location.reload();
            }
        });
}

function clearCookies() {
    const cookies = document.cookie.split("; ");
    for (let i = 0; i < cookies.length; i++) {
        const cookie = cookies[i];
        const eqPos = cookie.indexOf("=");
        const name = eqPos > -1 ? cookie.substr(0, eqPos) : cookie;
        document.cookie = name + "=;expires=Thu, 01 Jan 1970 00:00:00 GMT;path=/";
    }
}
