#include <stdint.h>

/* PS UART0はPYNQ-Z1のPROG UARTへMIO 14/15で接続されている。 */
#define UART0_BASE       0xe0000000u
#define UART_CONTROL     (*(volatile uint32_t *)(UART0_BASE + 0x00u))
#define UART_MODE        (*(volatile uint32_t *)(UART0_BASE + 0x04u))
#define UART_BAUDGEN     (*(volatile uint32_t *)(UART0_BASE + 0x18u))
#define UART_STATUS      (*(volatile uint32_t *)(UART0_BASE + 0x2cu))
#define UART_FIFO        (*(volatile uint32_t *)(UART0_BASE + 0x30u))
#define UART_BAUDDIV     (*(volatile uint32_t *)(UART0_BASE + 0x34u))

/* GPIO 0～9はPL出力の受信、16～25はPL入力への送信に使う。 */
#define GPIO_BASE        0xe000a000u
#define GPIO_DATA_BANK2  (*(volatile uint32_t *)(GPIO_BASE + 0x48u))
#define GPIO_INPUT_BANK2 (*(volatile uint32_t *)(GPIO_BASE + 0x68u))
#define GPIO_DIR_BANK2   (*(volatile uint32_t *)(GPIO_BASE + 0x284u))
#define GPIO_OUT_BANK2   (*(volatile uint32_t *)(GPIO_BASE + 0x288u))
#define GPIO_TX_ACK      (1u << 9)
#define GPIO_RX_SHIFT    16u
#define GPIO_RX_TOGGLE   (1u << 24)
#define GPIO_RX_ACK      (1u << 25)
#define GPIO_OUTPUT_MASK (GPIO_TX_ACK | (0x1ffu << GPIO_RX_SHIFT))

static void uart_init(void)
{
    UART_CONTROL = 0x0000002bu;
    UART_MODE = 0x00000020u;
    /* UART基準クロック100 MHz: 100000000 / (124 * (6 + 1)) = 115207 baud */
    UART_BAUDGEN = 124u;
    UART_BAUDDIV = 6u;
    UART_CONTROL = 0x00000017u;
}

static void uart_putc(uint8_t value)
{
    while ((UART_STATUS & 0x10u) != 0u) {
    }
    UART_FIFO = value;
}

int main(void)
{
    uint32_t acknowledged_tx_toggle = 0u;
    uint32_t rx_toggle = 0u;
    uint32_t gpio_output = 0u;

    uart_init();
    /* bridge起動とPS UART経路をPL出力とは独立に確認できる固定マーカー。 */
    {
        static const char ready[] = "PS BRIDGE READY\r\n";
        uint32_t index;
        for (index = 0u; index < sizeof(ready) - 1u; index++) {
            uart_putc((uint8_t)ready[index]);
        }
    }
    GPIO_DATA_BANK2 = gpio_output;
    GPIO_DIR_BANK2 = GPIO_OUTPUT_MASK;
    GPIO_OUT_BANK2 = GPIO_OUTPUT_MASK;

    for (;;) {
        uint32_t mailbox = GPIO_INPUT_BANK2;
        uint32_t tx_toggle = (mailbox >> 8) & 1u;

        if (tx_toggle != acknowledged_tx_toggle) {
            uint8_t value = (uint8_t)mailbox;
            if (value == 10u) {
                uart_putc(13u);
            }
            uart_putc(value);
            acknowledged_tx_toggle = tx_toggle;
            gpio_output = (gpio_output & ~GPIO_TX_ACK) |
                (acknowledged_tx_toggle << 9);
            GPIO_DATA_BANK2 = gpio_output;
        }

        if ((UART_STATUS & 0x02u) == 0u &&
                ((mailbox & GPIO_RX_ACK) != 0u) == (rx_toggle != 0u)) {
            uint8_t value = (uint8_t)UART_FIFO;
            rx_toggle ^= 1u;
            gpio_output &= ~(0x1ffu << GPIO_RX_SHIFT);
            gpio_output |= (uint32_t)value << GPIO_RX_SHIFT;
            gpio_output |= rx_toggle << 24;
            GPIO_DATA_BANK2 = gpio_output;
        }
    }
}
