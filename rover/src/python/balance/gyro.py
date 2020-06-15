import smbus
import time


# Get I2C bus
bus = smbus.SMBus(1)


CTRL_REG1 = 0x20
CTRL_REG2 = 0x21
CTRL_REG3 = 0x22
CTRL_REG4 = 0x23
CTRL_REG5 = 0x24
REFERENCE = 0x25
OUT_TEMP = 0x26
STATUS_REG = 0x27
OUT_X_L = 0x28
OUT_X_H = 0x29
OUT_Y_L = 0x2A
OUT_Y_H = 0x2B
OUT_Z_L = 0x2C
OUT_Z_H = 0x2D
FIFO_CTRL_REG = 0x2E
FIFO_SRC_REG = 0x2F
INT1_CFG = 0x30
INT1_SRC = 0x31
INT1_TSH_XH = 0x32
INT1_TSH_XL = 0x33
INT1_TSH_YH = 0x34
INT1_TSH_YL = 0x35
INT1_TSH_ZH = 0x36
INT1_TSH_ZL = 0x37
INT1_DURATION = 0x38


FREQ_BANDWIDTH_100_12_5 = 0x00
FREQ_BANDWIDTH_100_25 = 0x10
FREQ_BANDWIDTH_200_12_5 = 0x40
FREQ_BANDWIDTH_200_25 = 0x50
FREQ_BANDWIDTH_200_50 = 0x60
FREQ_BANDWIDTH_200_70 = 0x70
FREQ_BANDWIDTH_400_20 = 0x80
FREQ_BANDWIDTH_400_25 = 0x90
FREQ_BANDWIDTH_400_50 = 0xA0
FREQ_BANDWIDTH_400_110 = 0xB0
FREQ_BANDWIDTH_800_30 = 0xC0
FREQ_BANDWIDTH_800_35 = 0xD0
FREQ_BANDWIDTH_800_50 = 0xE0
FREQ_BANDWIDTH_800_111 = 0xF0


class _DataPoint:
    def __init__(self):
        self.time = 0
        self.dx = 0
        self.dy = 0
        self.dz = 0
        self.status = 0
        self.fifo_status = 0

    def set_data(self, new_dx, new_dy, new_dz, status, fifo_status):
        self.time = time.time()
        self.dx = new_dx
        self.dy = new_dy
        self.dz = new_dz
        self.status = status
        self.fifo_status = fifo_status

    def get_deltas(self):
        return self.dx, self.dy, self.dz


class L3G4200D:
    ALLOWED_FREQ_BANDWIDTH_COMBINATIONS = {
        100: { '_': 0x00, 12.5: 0, 25: 0x10},
        200: {'_': 0x40, 12.5: 0, 25: 0x10, 50: 0x20, 70: 0x30},
        400: {'_': 0x80, 20: 0, 25: 0x10, 50: 0x20, 110: 0x30},
        800: {'_': 0xC0, 30: 0, 35: 0x10, 50: 0x20, 110: 0x30}
    }

    def __init__(self, address=0x69, freq=400, bandwidth=50):
        if freq not in self.ALLOWED_FREQ_BANDWIDTH_COMBINATIONS:
            raise ValueError("Fequency can be only one of: 100, 200, 400 or 800")

        if bandwidth not in self.ALLOWED_FREQ_BANDWIDTH_COMBINATIONS[freq]:
            raise ValueError(f"Bandwidth for frequency {freq} can be only one of: {[b for b in self.ALLOWED_FREQ_BANDWIDTH_COMBINATIONS[freq] if b != '_']}")

        self.address = address
        self.px = 0
        self.py = 0
        self.pz = 0
        self.freq = freq
        self.bandwidth = bandwidth
        self.cx = 0
        self.cy = 0
        self.cz = 0
        self.buffer_len_in_time = 10
        self.data_buffer = [_DataPoint()]
        self.is_idle = True

        self.filter = 0.3
        self.sensitivity = 0.0175  # introduce FS (sensitivity) as 250, 500 or 2000

        self.init_gyro()

    def init_gyro(self):
        ctrl1 = 0xf + self.ALLOWED_FREQ_BANDWIDTH_COMBINATIONS[self.freq]['_'] + self.ALLOWED_FREQ_BANDWIDTH_COMBINATIONS[self.freq][self.bandwidth]

        bus.write_byte_data(self.address, CTRL_REG1, ctrl1)  # Output data rate 800Hz, freq cut-off 50 (Hz?), normal mode (not power down), all axes (x, y, z) enabled
        bus.write_byte_data(self.address, CTRL_REG2, 0x0)
        bus.write_byte_data(self.address, CTRL_REG3, 0x0)
        bus.write_byte_data(self.address, CTRL_REG4, 0x20)  # Not block (continuous update), LSB @ lower address, FSR 500dps, self test disabled, i2c interface
        # bus.write_byte_data(self.address, CTRL_REG4, 0x30)  # Not block (continuous update), LSB @ lower address, FSR 2000dps, self test disabled, i2c interface
        bus.write_byte_data(self.address, CTRL_REG5, 0x40)  # FIFO enabled
        self.idle()

    def set_buffer_len(self, len_in_time):
        self.buffer_len_in_time = len_in_time

    def get_buffer_len(self):
        return self.buffer_len_in_time

    def idle(self):
        self.is_idle = True
        bus.write_byte_data(self.address, FIFO_CTRL_REG, 0x00)  # Bypass mode

    def start(self):
        bus.write_byte_data(self.address, FIFO_CTRL_REG, 0x60)  # FIFO Stream mode
        self.is_idle = False

    def calibrate(self, calibration_time):
        reads = 0
        self.cx = 0
        self.cy = 0
        self.cz = 0

        last_time = time.time() - calibration_time

        _x = 0
        _y = 0
        _z = 0

        i = len(self.data_buffer) - 1
        while i >= 0 and self.data_buffer[i].time >= last_time:
            dx, dy, dz = self.data_buffer[i].get_deltas()
            _x += dx
            _y += dy
            _z += dz
            reads += 1
            i -= 1

        self.cx = _x / reads
        self.cy = _y / reads
        self.cz = _z / reads

        return self.cx, self.cy, self.cz

    def reset_position(self, pos_x=0, pos_y=0, pos_z=0):
        self.px = pos_x
        self.py = pos_y
        self.pz = pos_z

    def get_position(self):
        return self.px, self.py, self.pz

    def read_deltas(self):
        def read_data(_status, _fifo_status, _cut_off_buf_time):
            if len(self.data_buffer) > 0 and self.data_buffer[0].time < _cut_off_buf_time:
                _data_point = self.data_buffer[0]
                del self.data_buffer[0]
            else:
                _data_point = _DataPoint()

            data = bus.read_i2c_block_data(self.address, OUT_X_L + 0x80, 6)

            _dx = data[1] * 256 + data[0]
            _dx = _dx - 65536 if _dx > 32767 else _dx
            _dy = data[3] * 256 + data[2]
            _dy = _dy - 65536 if _dy > 32767 else _dy
            _dz = data[5] * 256 + data[4]
            _dz = _dz - 65536 if _dz > 32767 else _dz

            _data_point.set_data(_dx, _dy, _dz, _status, _fifo_status)

            self.data_buffer.append(_data_point)
            return _data_point

        def sanitise_angle(angle):
            if angle >= 180.0:
                angle -= 360.0
            elif angle < -180.0:
                angle += 360.0
            return angle

        now = time.time()
        cut_off_buf_time = now - self.buffer_len_in_time

        waited_for_data = False
        status = bus.read_byte_data(self.address, STATUS_REG)

        while (status & 0xf) != 0xf and not self.is_idle:
            if time.time() - 1 > now:
                print(f"{now}:  Waited for 1s for data {bin(status)}")
            waited_for_data = True
            status = bus.read_byte_data(self.address, STATUS_REG)

        if waited_for_data:
            status += 256
        status += 1024  # Adding top most bit - so it always have all digits when converted

        fifo_status = bus.read_byte_data(self.address, FIFO_SRC_REG)

        result_data = []
        if self.is_idle:
            data_point = read_data(status, fifo_status, cut_off_buf_time)
            result_data.append(data_point)
        else:
            while fifo_status & 0x1f != 0:
                if time.time() - 1 > now:
                    print(f"Collected data for more than 1s {bin(fifo_status)}")
                data_point = read_data(status, fifo_status, cut_off_buf_time)
                result_data.append(data_point)
                fifo_status = bus.read_byte_data(self.address, FIFO_SRC_REG)

        for data_point in result_data:
            dx, dy, dz = data_point.get_deltas()

            _x = (dx - self.cx) * self.sensitivity
            _y = (dy - self.cy) * self.sensitivity
            _z = (dz - self.cz) * self.sensitivity

            self.px = _x * self.filter + (1 - self.filter) * self.px
            self.py = _y * self.filter + (1 - self.filter) * self.py
            self.pz = _z * self.filter + (1 - self.filter) * self.pz

        return result_data


if __name__ == "__main__":

    gyro = L3G4200D()

    # gyro.calibrate(10)

    print("\x1b[2J\x1b[0;0H")

    while True:
        gyro.read_deltas()
        x, y, z = gyro.get_position()
        print("\x1b[0;0H")
        print(f"Rotation in X-Axis : {x:>10}")
        print(f"Rotation in Y-Axis : {y:>10}")
        print(f"Rotation in Z-Axis : {z:>10}")
        print(f"FIFO status        : {hex(z):>10}")
