/*
 * mapper1.c
 *
 * Created: 3/10/2017 12:09:56 PM
 *  Author: tepperson
 */ 

#include "mapper1.h"

static uint8_t *prg_rom;
static uint32_t prg_rom_mask;

void mapper1_set_pgr_rom(uint8_t *data, uint32_t size)
{
	prg_rom = data;
	prg_rom_mask = size - 1;
};

uint8_t mapper1_read(uint16_t addr)
{
	return prg_rom[addr & prg_rom_mask];
}

void mapper1_write(uint16_t addr, uint8_t val)
{
	prg_rom[addr & prg_rom_mask] = val;
}