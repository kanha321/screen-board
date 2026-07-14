import os
import sys
import json
import time
import struct
import select
import queue
import threading
from http.server import SimpleHTTPRequestHandler, HTTPServer

# Linux input event struct format (64-bit)
# time (seconds): long long (q), time (microseconds): long long (q), type: unsigned short (H), code: unsigned short (H), value: signed int (i)
EVENT_FORMAT = 'qqHHi'
EVENT_SIZE = struct.calcsize(EVENT_FORMAT)

# Keep track of active SSE client queues
clients = []
clients_lock = threading.Lock()

def broadcast_event(event_data):
    with clients_lock:
        for q in clients:
            q.put(event_data)

def read_input_devices():
    """Background thread to read event258 and event259 and broadcast events."""
    pen_path = '/dev/input/event258'
    kbd_path = '/dev/input/event259'
    
    print("Device reader thread started.", flush=True)
    
    while True:
        f_pen, f_kbd = None, None
        try:
            # Attempt to open both event devices with buffering=0 (unbuffered)
            if os.path.exists(pen_path):
                f_pen = open(pen_path, 'rb', buffering=0)
            if os.path.exists(kbd_path):
                f_kbd = open(kbd_path, 'rb', buffering=0)
                
            if not f_pen and not f_kbd:
                print("Gaomon WH851 devices not found. Retrying in 3 seconds...", flush=True)
                time.sleep(3)
                continue
                
            print("Successfully opened Gaomon WH851 input devices. Listening...", flush=True)
            
            inputs = {}
            if f_pen:
                inputs[f_pen] = "PEN"
            if f_kbd:
                inputs[f_kbd] = "KEYBOARD"
                
            # Dictionary to track last seen pen state to minimize duplicate updates
            pen_state = {"x": 0, "y": 0, "pressure": 0, "tilt_x": 0, "tilt_y": 0}
            
            while True:
                r, w, x = select.select(inputs.keys(), [], [], 1.0)
                if not r:
                    continue
                    
                pen_changed = False
                
                for f in r:
                    data = f.read(EVENT_SIZE)
                    if not data:
                        continue
                        
                    sec, usec, etype, code, value = struct.unpack(EVENT_FORMAT, data)
                    device = inputs[f]
                    
                    if device == "PEN":
                        # Absolute coordinates & pressure
                        if etype == 3:  # EV_ABS
                            if code == 0:     # ABS_X
                                pen_state["x"] = value
                                pen_changed = True
                            elif code == 1:   # ABS_Y
                                pen_state["y"] = value
                                pen_changed = True
                            elif code == 24:  # ABS_PRESSURE
                                pen_state["pressure"] = value
                                pen_changed = True
                            elif code == 26:  # ABS_TILT_X
                                pen_state["tilt_x"] = value
                                pen_changed = True
                            elif code == 27:  # ABS_TILT_Y
                                pen_state["tilt_y"] = value
                                pen_changed = True
                        elif etype == 1:  # EV_KEY
                            # BTN_TOUCH (330) or BTN_TOOL_PEN (320) or stylus buttons
                            broadcast_event({
                                "device": "PEN",
                                "type": "key",
                                "code": code,
                                "value": value
                            })
                            
                    elif device == "KEYBOARD":
                        # Tablet Express Keys (EV_KEY)
                        if etype == 1:  # EV_KEY
                            broadcast_event({
                                "device": "KEYBOARD",
                                "type": "key",
                                "code": code,
                                "value": value
                            })
                
                if pen_changed:
                    broadcast_event({
                        "device": "PEN",
                        "type": "motion",
                        **pen_state
                    })
                    
        except Exception as e:
            print(f"Error reading devices: {e}. Reconnecting...", flush=True)
            time.sleep(2)
        finally:
            if f_pen:
                f_pen.close()
            if f_kbd:
                f_kbd.close()

class CustomHTTPRequestHandler(SimpleHTTPRequestHandler):
    def do_GET(self):
        if self.path == '/events':
            self.send_response(200)
            self.send_header('Content-Type', 'text/event-stream')
            self.send_header('Cache-Control', 'no-cache')
            self.send_header('Connection', 'keep-alive')
            self.send_header('Access-Control-Allow-Origin', '*')
            self.end_headers()
            
            client_queue = queue.Queue()
            with clients_lock:
                clients.append(client_queue)
                
            print(f"New client connected. Total clients: {len(clients)}", flush=True)
            
            try:
                while True:
                    try:
                        # Get event from queue with 2 sec timeout
                        event_data = client_queue.get(timeout=2.0)
                        self.wfile.write(f"data: {json.dumps(event_data)}\n\n".encode('utf-8'))
                        self.wfile.flush()
                    except queue.Empty:
                        # Keep-alive heartbeat
                        self.wfile.write(b": keep-alive\n\n")
                        self.wfile.flush()
            except Exception as e:
                # Client disconnected
                pass
            finally:
                with clients_lock:
                    clients.remove(client_queue)
                print(f"Client disconnected. Total clients: {len(clients)}", flush=True)
        else:
            # Make sure we serve static files from the script's directory
            super().do_GET()

def run_server():
    # Change directory to the web folder to serve files properly
    script_dir = os.path.dirname(os.path.realpath(__file__))
    os.chdir(script_dir)
    
    server_address = ('', 8080)
    httpd = HTTPServer(server_address, CustomHTTPRequestHandler)
    print("Web Visualizer Server running at http://localhost:8080", flush=True)
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        print("\nShutting down server...", flush=True)
        httpd.server_close()

if __name__ == '__main__':
    # Start the input device reader in a daemon thread
    t = threading.Thread(target=read_input_devices, daemon=True)
    t.start()
    
    # Run the web server
    run_server()
