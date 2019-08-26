/*
 * nes.c
 *
 * Created: 2/10/2017 11:09:10 AM
 *  Author: tepperson
 */ 

#include <stdint.h>

#include "../lcd.h"
#include "cartridge.h"
#include "nes.h"
#include "processor_6502.h"

uint8_t ntsc_pal = 0;	//0 = ntsc, 1 = pal

void nes_insert_cartridge(unsigned char *img, uint32_t length)
{
	
}

void nes_remove_cartridge(void)
{
	
}

void nes_power_on(void)
{
	p6502_power_on();
}

void nes_reset(void)
{
	
}

void nes_reset_power_off(void)
{
	//hx8347a_fill(rgb24_to_rgb16(COLOR_BLACK));
}

void nes_run_tests(void)
{
	p6502_single_step = 1;
	nes_reset_power_off();
	nes_remove_cartridge();
	//load_cartridge_image(nestest, sizeof(nestest));
	psram_to_lcd();
	nes_power_on();
}

