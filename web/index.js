// Gaomon WH851 Web Visualizer Front-end logic

// Max hardware bounds for Gaomon WH851 digitizer
const MAX_X = 40640;
const MAX_Y = 25400;
const MAX_PRESSURE = 16384;

// DOM Elements
const statusDot = document.getElementById('statusDot');
const statusText = document.getElementById('statusText');
const valX = document.getElementById('valX');
const valY = document.getElementById('valY');
const valPressure = document.getElementById('valPressure');
const valTilt = document.getElementById('valTilt');
const pressureBar = document.getElementById('pressureBar');
const comboDisplay = document.getElementById('comboDisplay');
const logList = document.getElementById('logList');
const clearCanvasBtn = document.getElementById('clearCanvasBtn');

const drawCanvas = document.getElementById('drawCanvas');
const ctx = drawCanvas.getContext('2d');
const virtualTablet = document.getElementById('virtualTablet');
const digitizerArea = document.getElementById('digitizerArea');
const virtualStylus = document.getElementById('virtualStylus');

// Virtual Stylus Buttons
const btnStylus1 = document.getElementById('btn_stylus1');
const btnStylus2 = document.getElementById('btn_stylus2');

// Tracking active keycodes for combination checks
const activeKeys = new Set();

// Drawing State
let lastCanvasX = 0;
let lastCanvasY = 0;
let isDrawing = false;

// Handle Canvas Resizing dynamically
function resizeCanvas() {
    drawCanvas.width = digitizerArea.clientWidth;
    drawCanvas.height = digitizerArea.clientHeight;
    
    // Clear and redraw background details
    ctx.strokeStyle = '#222';
    ctx.lineWidth = 1;
}
window.addEventListener('resize', resizeCanvas);
resizeCanvas();

// Clear Canvas Button Action
clearCanvasBtn.addEventListener('click', () => {
    ctx.clearRect(0, 0, drawCanvas.width, drawCanvas.height);
    addLogMessage("Canvas cleared", "sys-msg");
});

// Add message to Event Logger list
function addLogMessage(msg, className = "") {
    const li = document.createElement('li');
    li.textContent = `[${new Date().toLocaleTimeString()}] ${msg}`;
    if (className) {
        li.className = className;
    }
    logList.appendChild(li);
    logList.scrollTop = logList.scrollHeight;
    
    // Limit list to 50 items to keep performance high
    while (logList.children.length > 50) {
        logList.removeChild(logList.firstChild);
    }
}

// Map keycode to friendly names
const keyMap = {
    18: "E",
    23: "I",
    26: "[",
    27: "]",
    29: "Ctrl",
    44: "Z",
    48: "B",
    56: "Alt",
    57: "Space"
};

// Map express keys elements
const expressKeyElements = {
    1: document.getElementById('btn_btn1'),
    2: document.getElementById('btn_btn2'),
    3: document.getElementById('btn_btn3'),
    4: document.getElementById('btn_btn4'),
    5: document.getElementById('btn_btn5'),
    6: document.getElementById('btn_btn6'),
    7: document.getElementById('btn_btn7'),
    8: document.getElementById('btn_btn8')
};

// Reset all button lights
function clearAllButtonHighlights() {
    Object.values(expressKeyElements).forEach(el => el.classList.remove('active'));
    btnStylus1.classList.remove('active');
    btnStylus2.classList.remove('active');
}

// Check key state and update active buttons
function updateButtonsHighlight() {
    clearAllButtonHighlights();
    
    // Check combos first
    if (activeKeys.has(29) && activeKeys.has(56) && activeKeys.has(44)) {
        // Ctrl + Alt + Z -> Button 8
        expressKeyElements[8].classList.add('active');
        comboDisplay.textContent = "Ctrl + Alt + Z (K8)";
    } else if (activeKeys.has(29) && activeKeys.has(18)) {
        // Ctrl + E -> Button 7
        expressKeyElements[7].classList.add('active');
        comboDisplay.textContent = "Ctrl + E (K7)";
    } else {
        // Single keys
        let keysText = [];
        activeKeys.forEach(k => {
            if (keyMap[k]) keysText.push(keyMap[k]);
            else keysText.push(`Code ${k}`);
        });
        
        comboDisplay.textContent = keysText.join(" + ") || "None";
        
        // Single Express Key activations
        if (activeKeys.has(48)) {
            // Key B -> Button 1 & Stylus 1
            expressKeyElements[1].classList.add('active');
            btnStylus1.classList.add('active');
        }
        if (activeKeys.has(18)) {
            // Key E -> Button 2 & Stylus 2
            expressKeyElements[2].classList.add('active');
            btnStylus2.classList.add('active');
        }
        if (activeKeys.has(57)) {
            // Space -> Button 3
            expressKeyElements[3].classList.add('active');
        }
        if (activeKeys.has(23)) {
            // Key I -> Button 4
            expressKeyElements[4].classList.add('active');
        }
        if (activeKeys.has(26)) {
            // [ -> Button 5
            expressKeyElements[5].classList.add('active');
        }
        if (activeKeys.has(27)) {
            // ] -> Button 6
            expressKeyElements[6].classList.add('active');
        }
    }
}

// Connect to SSE Endpoint
let eventSource = null;

function connectToEvents() {
    statusDot.className = "status-dot disconnected";
    statusText.textContent = "Connecting to Server...";
    
    eventSource = new EventSource('/events');
    
    eventSource.onopen = function() {
        statusDot.className = "status-dot connected";
        statusText.textContent = "Connected to Gaomon WH851";
        addLogMessage("Connected to server stream", "sys-msg");
    };
    
    eventSource.onerror = function() {
        statusDot.className = "status-dot disconnected";
        statusText.textContent = "Disconnected. Retrying...";
        eventSource.close();
        setTimeout(connectToEvents, 2000);
    };
    
    eventSource.onmessage = function(event) {
        // Ignore heartbeats
        if (event.data.trim() === "" || event.data.startsWith(":")) return;
        
        try {
            const data = JSON.parse(event.data);
            
            // Handle KEY/BUTTON Events
            if (data.type === 'key') {
                const isDown = data.value === 1 || data.value === 2; // 1 is press, 2 is repeat
                const keyName = keyMap[data.code] || `Key ${data.code}`;
                
                if (isDown) {
                    activeKeys.add(data.code);
                    if (data.value === 1) {
                        addLogMessage(`${data.device} Press: ${keyName}`, "key-press");
                    }
                } else {
                    activeKeys.discard ? activeKeys.discard(data.code) : activeKeys.delete(data.code);
                    addLogMessage(`${data.device} Release: ${keyName}`, "key-release");
                }
                
                updateButtonsHighlight();
            }
            
            // Handle Stylus Hover & Move Events
            else if (data.type === 'motion') {
                const xPercent = (data.x / MAX_X) * 100;
                const yPercent = (data.y / MAX_Y) * 100;
                
                // Update text telemetry
                valX.textContent = data.x;
                valY.textContent = data.y;
                valPressure.textContent = `${data.pressure} / ${MAX_PRESSURE}`;
                valTilt.textContent = `${data.tilt_x}° / ${data.tilt_y}°`;
                
                // Update pressure indicator bar
                const pressurePercentage = (data.pressure / MAX_PRESSURE) * 100;
                pressureBar.style.width = `${pressurePercentage}%`;
                
                // Update virtual stylus position
                virtualStylus.style.left = `${xPercent}%`;
                virtualStylus.style.top = `${yPercent}%`;
                virtualStylus.classList.add('in-range');
                
                // Tilt styling
                virtualStylus.style.transform = `translate(-50%, 0px) rotate(${data.tilt_x / 4}deg)`;
                
                // Map coordinates to canvas pixels
                const canvasX = (data.x / MAX_X) * drawCanvas.width;
                const canvasY = (data.y / MAX_Y) * drawCanvas.height;
                
                if (data.pressure > 0) {
                    virtualStylus.classList.add('touching');
                    
                    // Draw on canvas
                    if (isDrawing) {
                        ctx.beginPath();
                        ctx.moveTo(lastCanvasX, lastCanvasY);
                        ctx.lineTo(canvasX, canvasY);
                        
                        // Width relative to pressure
                        ctx.lineWidth = 1 + (data.pressure / MAX_PRESSURE) * 14;
                        ctx.lineCap = 'round';
                        ctx.lineJoin = 'round';
                        
                        // Neon drawing color gradient
                        ctx.strokeStyle = `hsl(${(180 + (data.pressure / MAX_PRESSURE) * 120) % 360}, 100%, 50%)`;
                        ctx.stroke();
                    }
                    isDrawing = true;
                } else {
                    virtualStylus.classList.remove('touching');
                    isDrawing = false;
                }
                
                lastCanvasX = canvasX;
                lastCanvasY = canvasY;
            }
            
        } catch (e) {
            console.error("Error parsing event data:", e, event.data);
        }
    };
}

// Start connection
connectToEvents();
