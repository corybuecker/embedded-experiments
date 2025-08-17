```sh
python -m venv .venv

source .venv/bin/activate

pip install west

west init

west update

west zephyr-export

west packages pip --install

cd zephyr

west sdk install -t arm-zephyr-eabi
```