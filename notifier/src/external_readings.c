#include "external_readings.h"

#define SW0_NODE DT_ALIAS(sw0)
const struct gpio_dt_spec button = GPIO_DT_SPEC_GET_OR(SW0_NODE, gpios, {0});
static struct gpio_callback button_press_callback;

void button_pressed(const struct device *port, struct gpio_callback *cb,
                    gpio_port_pins_t pins) {
  store_reading(1);
}

int initialize_gpio_readings() {
  if (!gpio_is_ready_dt(&button)) {
    return -1;
  }

  int err = gpio_pin_configure_dt(&button, GPIO_INPUT);
  if (err != 0) {
    return err;
  }

  err = gpio_pin_interrupt_configure_dt(&button, GPIO_INT_EDGE_TO_ACTIVE);
  if (err != 0) {
    return err;
  }

  gpio_init_callback(&button_press_callback, button_pressed, BIT(button.pin));
  gpio_add_callback(button.port, &button_press_callback);

  return 0;
}
