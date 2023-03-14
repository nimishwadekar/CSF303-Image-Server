from socket import *

def main():
    with socket(AF_INET, SOCK_STREAM) as sock:
        sock.connect(('127.0.0.1', 4000))
        sock.sendall(b'HELO 2019A7PS1004G')
        cmd = sock.recv(5)
        print(cmd)
        if cmd != b'SIZE ':
            print('size not received')
            return
        
        buf = sock.recv(32)
        size = int(buf)
        print(size)

        sock.sendall(b'FILE ')

        cmd = sock.recv(5)
        print(cmd)
        if cmd != b'DATA ':
            print('data not received')
            return

        data = []

        # maybe give hint about how to repeatedly read large files

        while size > 0:
            buf = sock.recv(4096)
            size -= len(buf)
            data.append(buf)

        sum = 0
        checksum = 0
        for buf in data:
            sum += len(buf)
            for byte in buf:
                checksum += byte
        checksum &= 0xFF
        checksum = (~checksum) + 1
        if checksum < 0:
            checksum += 256
        
        print(sum)
        print(checksum)

        sock.sendall(bytes(f'ABRA {checksum}', 'ascii'))

        print(sock.recv(32))

        with open('./tmp/tmp_pic.png', 'wb') as f:
            f.truncate(0)
            for buf in data:
                f.write(buf)

if __name__ == '__main__':
    main()