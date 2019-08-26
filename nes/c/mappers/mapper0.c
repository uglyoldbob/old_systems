/*
 * mapper0.c
 *
 * Created: 2/14/2017 2:48:15 PM
 *  Author: tepperson
 */ 

#include "mapper0.h"

static uint8_t *prg_rom;
static uint32_t prg_rom_mask;

void mapper0_set_pgr_rom(uint8_t *data, uint32_t size)
{
	prg_rom = data;
	prg_rom_mask = size - 1;
};

uint8_t mapper0_read(uint16_t addr)
{
	return prg_rom[addr & prg_rom_mask];
}

void mapper0_write(uint16_t addr, uint8_t val)
{
	prg_rom[addr & prg_rom_mask] = val;
}