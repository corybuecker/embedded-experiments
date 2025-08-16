//
// Created by Cory Buecker on 8/12/25.
//

#include "storage.h"

K_HEAP_DEFINE(storage, REQUIRED_MEMORY);

static external_reading_t *head;
static external_reading_t *tail;

static struct k_mutex mutex;
static bool is_initialized = false;

void initialize_storage()
{
    if (is_initialized)
    {
        return;
    }
    uint8_t current_reading_position = 0;
    external_reading_t *current_node =
        k_heap_alloc(&storage, sizeof(external_reading_t), K_FOREVER);
    current_node->value = 0;
    head = current_node;

    while (current_reading_position < MAXIMUM_STORED_READINGS)
    {
        external_reading_t *next_node =
            k_heap_alloc(&storage, sizeof(external_reading_t), K_FOREVER);
        next_node->value = 0;
        current_node->next = next_node;
        current_node = current_node->next;
        current_reading_position = current_reading_position + 1;
    }

    tail = current_node;

    printk("initializing mutex for storage\n");
    k_mutex_init(&mutex);
    is_initialized = true;
}

void store_reading(const uint8_t value)
{
    if (k_mutex_lock(&mutex, K_MSEC(100)) != 0)
    {
        printf("Failed to lock mutex, aborting storage\n");
        return;
    }

    external_reading_t *previous_head = head;
    head = head->next;
    k_heap_free(&storage, previous_head);

    external_reading_t *new_reading =
        k_heap_alloc(&storage, sizeof(external_reading_t), K_FOREVER);
    new_reading->value = value;

    tail->next = new_reading;
    tail = new_reading;

    k_mutex_unlock(&mutex);
}
