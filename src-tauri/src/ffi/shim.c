/*
 * Minimal C shim around cubiomes.
 *
 * All cubiomes structs (Generator, SurfaceNoise, Pos, ...) stay on the C side
 * so Rust never has to reproduce their layout. Rust only sees opaque pointers
 * and plain integers/floats.
 */

#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "biomenoise.h"
#include "biomes.h"
#include "finders.h"
#include "generator.h"
#include "stronghold.h"
#include "util.h"

void *sf_gen_new(int mc, uint32_t flags)
{
    Generator *g = (Generator *)calloc(1, sizeof(Generator));
    if (g == NULL)
        return NULL;
    setupGenerator(g, mc, flags);
    return (void *)g;
}

void sf_gen_free(void *gen)
{
    free(gen);
}

void sf_apply_seed(void *gen, int dim, uint64_t seed)
{
    applySeed((Generator *)gen, dim, seed);
}

void sf_get_spawn(const void *gen, int *out_x, int *out_z)
{
    Pos pos = getSpawn((const Generator *)gen);
    if (out_x != NULL)
        *out_x = pos.x;
    if (out_z != NULL)
        *out_z = pos.z;
}

int sf_get_biome_at(const void *gen, int scale, int x, int y, int z)
{
    return getBiomeAt((const Generator *)gen, scale, x, y, z);
}

void *sf_noise_new(int dim, uint64_t seed)
{
    SurfaceNoise *sn = (SurfaceNoise *)calloc(1, sizeof(SurfaceNoise));
    if (sn == NULL)
        return NULL;
    initSurfaceNoise(sn, dim, seed);
    return (void *)sn;
}

void sf_noise_free(void *noise)
{
    free(noise);
}

/* Floor division (C truncates toward zero, which is wrong for negative x/y). */
static int sf_floordiv(int a, int b)
{
    int q = a / b;
    if (a % b != 0 && ((a < 0) != (b < 0)))
        q--;
    return q;
}

int sf_map_approx_height(float *y, int *ids, const void *gen, const void *noise,
    int x, int z, int w, int h)
{
    return mapApproxHeight(y, ids, (const Generator *)gen,
        (const SurfaceNoise *)noise, x, z, w, h);
}

const char *sf_biome_name(int mc, int id)
{
    return biome2str(mc, id);
}

int sf_mc_from_string(const char *name)
{
    return str2mc(name);
}

int sf_biome_exists(int mc, int id)
{
    return biomeExists(mc, id);
}

int sf_is_overworld(int mc, int id)
{
    return isOverworld(mc, id);
}

int sf_is_oceanic(int id)
{
    return isOceanic(id);
}

/*
 * Fills `ids` with `w * h` biome ids at a native cubiomes scale
 * (4 = 1:4, 16 = 1:16, 64 = 1:64) sampled at the block height `y`.
 * `x`/`z` are coordinates in that scale, `y` is in blocks. Returns 0 on
 * success. A fixed height below the surface is what makes cave biomes
 * (lush caves, dripstone caves, deep dark) visible from above.
 */
int sf_biome_grid_at(int *ids, const void *gen, int scale, int x, int y, int z,
    int w, int h)
{
    const Generator *g = (const Generator *)gen;
    if (g == NULL || ids == NULL || w <= 0 || h <= 0)
        return -1;
    if (scale != 4 && scale != 16 && scale != 64)
        return -1;

    /* Range y is given in scale units, so the block Y is divided down. */
    Range r = {scale, x, z, w, h, sf_floordiv(y, scale), 1};
    int *cache = allocCache(g, r);
    if (cache == NULL)
        return -1;

    int err = genBiomes(g, cache, r);
    if (err == 0)
        memcpy(ids, cache, sizeof(int) * (size_t)w * (size_t)h);
    free(cache);
    return err;
}

/*
 * Fills `ids` with `w * h` biome ids at a native cubiomes scale, sampled at
 * the top of the world (surface / top-down view, no cave biomes).
 * `x`/`z` are coordinates in that scale. Returns 0 on success.
 */
int sf_biome_grid(int *ids, const void *gen, int scale, int x, int z, int w, int h)
{
    const Generator *g = (const Generator *)gen;
    if (g == NULL)
        return -1;

    int surface = g->mc >= MC_1_18 ? 300 : 255;
    return sf_biome_grid_at(ids, gen, scale, x, surface, z, w, h);
}

int sf_structure_supported(int mc, int type)
{
    StructureConfig conf;
    return getStructureConfig(type, mc, &conf);
}

/*
 * Finds the nearest viable structure of `type` within `radius` blocks of
 * (cx, cz). The generator must already have the seed applied. Returns 1 and
 * writes the block position when a structure was found, 0 otherwise.
 *
 * Regions are walked in rings around the center and the search stops as soon
 * as no outer ring can beat the closest hit. Dense kinds such as treasure use
 * one region per chunk, so a full bounding-box scan would be prohibitively
 * large; the ring walk keeps them searchable at any radius.
 */
int sf_structure_near(const void *gen, int type, int cx, int cz, int radius,
    int *out_x, int *out_z)
{
    const Generator *g = (const Generator *)gen;
    StructureConfig conf;
    if (g == NULL || out_x == NULL || out_z == NULL || radius <= 0
        || !getStructureConfig(type, g->mc, &conf))
        return 0;

    int region_blocks = conf.regionSize * 16;
    if (region_blocks <= 0)
        return 0;

    int crx = sf_floordiv(cx, region_blocks);
    int crz = sf_floordiv(cz, region_blocks);
    int max_ring = radius / region_blocks + 2;
    int64_t radius_sq = (int64_t)radius * radius;
    int64_t best = INT64_MAX;
    int found = 0;
    int scanned = 0;

    for (int ring = 0; ring <= max_ring; ring++)
    {
        /* Every region in this ring or farther is at least this close. */
        if (found)
        {
            int64_t lower = (int64_t)(ring - 1) * region_blocks;
            if (lower > 0 && lower * lower > best)
                break;
        }

        for (int dz = -ring; dz <= ring; dz++)
        {
            /* Interior rows only need their two corners. */
            int step = (ring > 0 && dz != -ring && dz != ring) ? 2 * ring : 1;
            for (int dx = -ring; dx <= ring; dx += step)
            {
                if (++scanned > 200000)
                    return found;

                Pos pos;
                if (!getStructurePos(type, g->mc, g->seed, crx + dx, crz + dz, &pos))
                    continue;
                int64_t ddx = (int64_t)pos.x - cx;
                int64_t ddz = (int64_t)pos.z - cz;
                int64_t dist = ddx * ddx + ddz * ddz;
                if (dist > radius_sq)
                    continue;
                if (!isViableStructurePos(type, (Generator *)g, pos.x, pos.z, 0))
                    continue;
                if (dist < best)
                {
                    best = dist;
                    *out_x = pos.x;
                    *out_z = pos.z;
                    found = 1;
                }
            }
        }
    }
    return found;
}

/*
 * Collects every viable structure of `type` whose block position lies inside
 * the rectangle (x0, z0)-(x1, z1). Writes at most `max` positions. Returns the
 * number of positions written; kinds that would need an unreasonable number of
 * regions (e.g. per-chunk treasure at world scale) are skipped with 0.
 */
int sf_structures_in_area(const void *gen, int type, int x0, int z0, int x1, int z1,
    int *out_x, int *out_z, int max)
{
    const Generator *g = (const Generator *)gen;
    StructureConfig conf;
    if (g == NULL || out_x == NULL || out_z == NULL || max <= 0)
        return 0;
    if (x0 > x1 || z0 > z1)
        return 0;
    if (!getStructureConfig(type, g->mc, &conf))
        return 0;

    int region_blocks = conf.regionSize * 16;
    if (region_blocks <= 0)
        return 0;

    int rx0 = sf_floordiv(x0, region_blocks) - 1;
    int rx1 = sf_floordiv(x1, region_blocks) + 1;
    int rz0 = sf_floordiv(z0, region_blocks) - 1;
    int rz1 = sf_floordiv(z1, region_blocks) + 1;

    int64_t regions = (int64_t)(rx1 - rx0 + 1) * (int64_t)(rz1 - rz0 + 1);
    if (regions <= 0)
        return 0;

    /* Dense kinds (treasure: one region per chunk) would need millions of
       lookups on an overview map. Sample every n-th region instead of
       returning nothing, which still marks representative positions. */
    int stride = 1;
    while (regions / ((int64_t)stride * stride) > 4096)
        stride++;

    int count = 0;
    for (int rz = rz0; rz <= rz1; rz += stride)
    {
        for (int rx = rx0; rx <= rx1; rx += stride)
        {
            Pos pos;
            if (!getStructurePos(type, g->mc, g->seed, rx, rz, &pos))
                continue;
            if (pos.x < x0 || pos.x > x1 || pos.z < z0 || pos.z > z1)
                continue;
            if (!isViableStructurePos(type, (Generator *)g, pos.x, pos.z, 0))
                continue;
            out_x[count] = pos.x;
            out_z[count] = pos.z;
            if (++count >= max)
                return count;
        }
    }
    return count;
}

/*
 * Strongholds inside the rectangle, using cubiomes' ring iterator. Returns the
 * number of positions written (at most `max`).
 */
int sf_strongholds(const void *gen, int x0, int z0, int x1, int z1,
    int *out_x, int *out_z, int max)
{
    const Generator *g = (const Generator *)gen;
    if (g == NULL || out_x == NULL || out_z == NULL || max <= 0)
        return 0;

    StrongholdIter sh;
    initFirstStronghold(&sh, g->mc, g->seed);

    int count = 0;
    for (int guard = 0; guard < 128; guard++)
    {
        int remaining = nextStronghold(&sh, g);
        int x = sh.pos.x;
        int z = sh.pos.z;
        if (x >= x0 && x <= x1 && z >= z0 && z <= z1)
        {
            out_x[count] = x;
            out_z[count] = z;
            if (++count >= max)
                return count;
        }
        if (remaining <= 0)
            break;
    }
    return count;
}

int sf_slime_chunk(uint64_t seed, int chunk_x, int chunk_z)
{
    return isSlimeChunk(seed, chunk_x, chunk_z);
}

/*
 * Best end portal among the first `max_strongholds` strongholds.
 *
 * cubiomes writes the portal room eye state into Piece.additionalData:
 * bits 0..11 = eyes already placed, bit 12 = portal is lit (any colliding
 * chunk passed the twelve-eye check). All cubiomes structs stay on the C side.
 *
 * Returns 1 when a portal room was found and writes the eye count, lit flag
 * and the stronghold position; 0 otherwise.
 */
int sf_portal_eyes(const void *gen, int max_strongholds,
    int *out_eyes, int *out_lit, int *out_x, int *out_z)
{
    const Generator *g = (const Generator *)gen;
    if (g == NULL || max_strongholds <= 0)
        return 0;

    StructureSaltConfig ssconf;
    if (!getStructureSaltConfig(Stronghold, g->mc, -1, &ssconf))
        return 0;

    /* 400 pieces are enough for any realistic stronghold (cubiomes docs). */
    Piece *pieces = (Piece *)calloc(400, sizeof(Piece));
    if (pieces == NULL)
        return 0;

    StrongholdIter sh;
    initFirstStronghold(&sh, g->mc, g->seed);

    int best_eyes = -1;
    int best_lit = 0;
    int best_x = 0;
    int best_z = 0;

    for (int i = 0; i < max_strongholds; ++i)
    {
        int remaining = nextStronghold(&sh, (Generator *)g);
        StructureVariant sv;
        memset(&sv, 0, sizeof(sv));

        int count = getStructurePieces(pieces, 400, Stronghold, ssconf, &sv,
            g->mc, g->seed, sh.pos.x, sh.pos.z);
        if (count > 0)
        {
            for (int j = 0; j < count; ++j)
            {
                Piece *p = &pieces[j];
                if (p->type != SH_PORTAL_ROOM)
                    continue;

                int eyes = 0;
                for (int bit = 0; bit < 12; ++bit)
                    if (p->additionalData & (1 << bit))
                        eyes++;
                int lit = (p->additionalData & (1 << 12)) != 0;

                if (eyes > best_eyes || (eyes == best_eyes && lit && !best_lit))
                {
                    best_eyes = eyes;
                    best_lit = lit;
                    best_x = sh.pos.x;
                    best_z = sh.pos.z;
                }
            }
        }

        if (best_eyes == 12 && best_lit)
            break;
        if (remaining <= 0)
            break;
    }

    free(pieces);
    if (best_eyes < 0)
        return 0;

    if (out_eyes != NULL)
        *out_eyes = best_eyes;
    if (out_lit != NULL)
        *out_lit = best_lit;
    if (out_x != NULL)
        *out_x = best_x;
    if (out_z != NULL)
        *out_z = best_z;
    return 1;
}
