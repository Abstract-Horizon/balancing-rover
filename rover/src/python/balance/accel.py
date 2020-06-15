

import smbus
import math
import time

#
# As per https://morf.lv/mems-part-1-guide-to-using-accelerometer-adxl345
#

bus = smbus.SMBus(1)

# ADXL345 constants
EARTH_GRAVITY_MS2 = 9.80665
# SCALE_MULTIPLIER = 0.004
SCALE_MULTIPLIER = 0.00390625

DATA_FORMAT = 0x31
BW_RATE = 0x2C
POWER_CTL = 0x2D

BW_RATE_1600HZ = 0x0F
BW_RATE_800HZ = 0x0E
BW_RATE_400HZ = 0x0D
BW_RATE_200HZ = 0x0C
BW_RATE_100HZ = 0x0B
BW_RATE_50HZ = 0x0A
BW_RATE_25HZ = 0x09

RANGE_2G = 0x00
RANGE_4G = 0x01
RANGE_8G = 0x02
RANGE_16G = 0x03

MEASURE = 0x08
AXES_DATA = 0x32


class _DataPoint:
    def __init__(self):
        self.time = 0
        self.raw_x = 0
        self.raw_y = 0
        self.raw_z = 0

        self.x = 0
        self.y = 0
        self.z = 0

    def set_raw_data(self, new_dx, new_dy, new_dz):
        self.time = time.time()
        self.raw_x = new_dx
        self.raw_y = new_dy
        self.raw_z = new_dz

    def set_current_data(self, new_x, new_y, new_z):
        self.time = time.time()
        self.x = new_x
        self.y = new_y
        self.z = new_z

    def get_raw_data(self):
        return self.raw_x, self.raw_y, self.raw_z

    def get_data(self):
        return self.x, self.y, self.z


class ADXL345:

    ALLOWED_FREQUENCIES = {
        1600: BW_RATE_1600HZ,
        800: BW_RATE_800HZ,
        400: BW_RATE_400HZ,
        200: BW_RATE_200HZ,
        100: BW_RATE_100HZ,
        50: BW_RATE_50HZ,
        25: BW_RATE_25HZ
    }

    def __init__(self, address=0x53, freq=200):
        if freq not in self.ALLOWED_FREQUENCIES:
            raise ValueError(f"Frequency can be only one of: {[f for f in self.ALLOWED_FREQUENCIES]}")

        self.address = address
        self.x = 0
        self.y = 0
        self.z = 0

        self.x_offset = 0
        self.y_offset = 0
        self.z_offset = 0

        self.filter = 0.5

        self.inverse_freq = 1.0 / freq
        self.next_time = 0

        self.buffer_len_in_time = 10
        self.data_buffer = [_DataPoint()]

        self.set_bandwidth_rate(self.ALLOWED_FREQUENCIES[freq])
        self.set_range(RANGE_16G)
        self.enable_measurement()

    def enable_measurement(self):
        bus.write_byte_data(self.address, POWER_CTL, MEASURE)

    def set_bandwidth_rate(self, rate_flag):
        bus.write_byte_data(self.address, BW_RATE, rate_flag)

    # set the measurement range for 10-bit readings
    def set_range(self, range_flag):
        value = bus.read_byte_data(self.address, DATA_FORMAT)

        value &= ~0x0F
        value |= range_flag
        value |= 0x08  # FULL RES

        bus.write_byte_data(self.address, DATA_FORMAT, value)

    def calibrate(self, calibration_time):
        def fix_for_1g(offset):
            if offset > 0.5:
                offset -= 1.0
            if offset < -0.5:
                offset += 1.0
            return offset

        reads = 0
        self.x_offset = 0
        self.y_offset = 0
        self.z_offset = 0

        last_time = time.time() - calibration_time

        _x = 0
        _y = 0
        _z = 0

        i = len(self.data_buffer) - 1
        while i >= 0 and self.data_buffer[i].time >= last_time:
            dx, dy, dz = self.data_buffer[i].get_raw_data()
            _x += dx * SCALE_MULTIPLIER
            _y += dy * SCALE_MULTIPLIER
            _z += dz * SCALE_MULTIPLIER
            reads += 1
            i -= 1

        if reads > 0:
            self.x_offset = fix_for_1g(_x / reads)
            self.y_offset = fix_for_1g(_y / reads)
            self.z_offset = fix_for_1g(_z / reads)

        return self.x_offset, self.y_offset, self.z_offset

    def read_raw(self, wait_for_right_time=False):
        now = time.time()
        if wait_for_right_time:
            d_time = self.next_time - now
            while d_time > 0:
                if d_time > 0.02:
                    time.sleep(0.02)
                now = time.time()
                d_time = self.next_time - now

        if self.next_time == 0:
            self.next_time = now + self.inverse_freq
        else:
            self.next_time += self.inverse_freq

        read_bytes = bus.read_i2c_block_data(self.address, AXES_DATA, 6)

        _x = read_bytes[0] | (read_bytes[1] << 8)
        _x = _x - (1 << 16) if _x & (1 << 16 - 1) else _x

        _y = read_bytes[2] | (read_bytes[3] << 8)
        _y = _y - (1 << 16) if _y & (1 << 16 - 1) else _y

        _z = read_bytes[4] | (read_bytes[5] << 8)
        _z = _z - (1 << 16) if _z & (1 << 16 - 1) else _z

        cut_off_buf_time = now - self.buffer_len_in_time

        if len(self.data_buffer) > 0 and self.data_buffer[0].time < cut_off_buf_time:
            _data_point = self.data_buffer[0]
            del self.data_buffer[0]
        else:
            _data_point = _DataPoint()

        _data_point.set_raw_data(_x, _y, _z)
        self.data_buffer.append(_data_point)

        return _data_point

    def read(self):
        _data_point = self.read_raw()

        _x, _y, _z = _data_point.get_raw_data()

        _x = _x * SCALE_MULTIPLIER - self.x_offset
        _y = _y * SCALE_MULTIPLIER - self.y_offset
        _z = _z * SCALE_MULTIPLIER - self.z_offset

        self.x = _x * self.filter + self.x * (1.0 - self.filter)
        self.y = _y * self.filter + self.y * (1.0 - self.filter)
        self.z = _z * self.filter + self.z * (1.0 - self.filter)

        _data_point.set_current_data(self.x, self.y, self.z)

        return _data_point
    #
    # def read_pitch_roll(self):
    #     _data_point = self.read()
    #     _x, _y, _z = _data_point.get_data()
    #
    #     _data_point.pitch = (math.atan2(_x, math.sqrt(_x * _x + _y * _y)) * 180.0) / math.pi
    #     _data_point.roll = (math.atan2(_y, (math.sqrt(_z * _z + _y * _y))) * 180.0) / math.pi
    #
    #     return _data_point


if __name__ == "__main__":
    adxl345 = ADXL345()

    while True:
        x, y, z = adxl345.read().get_data()

        print("\x1b[0;0H")
        print(f"ADXL345 on address {adxl345.address}")
        print(f"   x = {x:>10.2f}G")
        print(f"   y = {y:>10.2f}G")
        print(f"   z = {z:>10.2f}G")
