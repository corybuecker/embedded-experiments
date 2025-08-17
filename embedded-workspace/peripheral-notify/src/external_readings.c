#include "external_readings.h"

#define SW0_NODE DT_ALIAS(sw0)
const struct gpio_dt_spec button = GPIO_DT_SPEC_GET_OR(SW0_NODE, gpios, {0});
static struct gpio_callback button_press_callback;

K_THREAD_STACK_DEFINE(gpio_sampling_thread_stack, 8192);
static struct k_thread gpio_sampling_thread;
static k_tid_t gpio_sampling_thread_id;

static bool gpio_initialized = false;

void button_pressed(const struct device *port, struct gpio_callback *cb,
                    gpio_port_pins_t pins) {
  store_reading(1);
}

int initialize_gpio_readings() {
  if (gpio_initialized) {
    return -1;
  }

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

  gpio_initialized = true;

  return 0;
}

void sample_gpio_and_store(void *p1, void *p2, void *p3) {
  while (true) {
    k_sleep(K_MSEC(250));

    store_reading(0);
  }
}

int initialize_gpio_sampling() {
  if (!gpio_initialized) {
    return -1;
  }

  gpio_sampling_thread_id = k_thread_create(
      &gpio_sampling_thread, gpio_sampling_thread_stack,
      K_THREAD_STACK_SIZEOF(gpio_sampling_thread_stack), sample_gpio_and_store,
      NULL, NULL, NULL, 12, 0, K_NO_WAIT);

  return 0;
}