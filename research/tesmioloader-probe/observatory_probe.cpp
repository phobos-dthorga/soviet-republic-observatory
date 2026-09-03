// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Republic Observatory contributors
// Optional same-process observation companion for TesmioLoader.
// This source writes only its own bounded telemetry file. It never writes game
// objects, save buffers, game files, Observatory databases, or the network.

#include "tesmio_plugin.h"

#define RVA_PERSON_VECTOR 0x9E75B8
#define RVA_GAME_STATE 0x9D4F10
#define OFF_DAY 0x590
#define OFF_YEAR 0x594
#define PERSON_SIZE 0x750
#define PERSON_CURRENT_BUILDING 0x20
#define PERSON_EDUCATION 0xA8
#define PERSON_AGE 0xD4
#define PERSON_STATUS 0xD8
#define PERSON_CLASS 0x71C
#define PERSON_MONEY_SPENT 0x734
#define RVA_RESOURCE_VECTOR 0x9E11C0
#define RVA_BUILDING_VECTOR 0x9E6A18
#define RESOURCE_STRIDE 0x340
#define RESOURCE_MAX_RECORDS 512
#define RESOURCE_TOKEN_BYTES 32
#define RESOURCE_CAPTION_ID 0x40
#define RESOURCE_KIND 0x44
#define RESOURCE_PRICE_RUB 0x58
#define RESOURCE_PRICE_USD 0x5C
#define RESOURCE_BASE_RUB 0x78
#define RESOURCE_BASE_USD 0x7C
#define RESOURCE_MARKET_RUB 0x88
#define RESOURCE_MARKET_USD 0xA8
#define RESOURCE_MARKET_SELL 0x00
#define RESOURCE_MARKET_BUY 0x04
#define RESOURCE_CLASS_BASE 0xCC
#define RESOURCE_CLASS_STRIDE 0x20
#define RESOURCE_CLASS_COUNT 18
#define RESOURCE_FAMILY 0x30C
#define TESTED_EXE_TIMESTAMP 0x6A3EB6ADu
// TesmioLoader exposes the loaded module's PE SizeOfImage, not the file length.
#define TESTED_EXE_SIZE 11128832u
#define SYM_TERRAIN_RENDER "?Render@C3D_TERRAIN@@QEAAX_NPEAVC3D_CAMERA@@0HH@Z"
#define PROBE_FILE "republic-observatory-probe.jsonl"
#define MAX_SAMPLES 32
#define MAX_STATUS_VALUE 1.5f
#define MAX_MONEY_SPENT 1.0e12f
#define FACILITY_CANDIDATE_CONTRACT 1
#define FACILITY_MAX_RECORDS 25000
#define FACILITY_PER_FRAME 128
#define BUILDING_TYPE_DESCRIPTOR 0x318
#define BUILDING_COMPLETION 0x604
#define BUILDING_STORAGES 0x970
#define BUILDING_GOING_AWAY 0xEA8
#define BUILDING_PRODUCTION 0xDDC
#define BUILDING_RESIDENTIAL_POLLUTION 0x11B0
#define BUILDING_TYPE 0x360
#define BUILDING_SUBTYPE 0x364
#define STORAGE_STRIDE 0xE0
#define STORAGE_CAPACITY 0x8C
#define STORAGE_SLOT_STRIDE 0x10
#define STORAGE_SLOT_RESOURCE 0x00
#define STORAGE_SLOT_CONTENT 0x08
#define STORAGE_SLOT_QUALITY 0x0C
#define CANDIDATE_FIELD_PRODUCTION 0x01u
#define CANDIDATE_FIELD_POLLUTION 0x02u
#define CANDIDATE_FIELD_WATER 0x04u
#define CANDIDATE_FIELD_SEWAGE 0x08u

typedef void (*t_TerrainRender)(void*, bool, void*, void*, int, int);
static t_TerrainRender o_TerrainRender;
static HANDLE g_output = INVALID_HANDLE_VALUE;
static int g_enabled = 1;
static int g_samples = 16;
static int g_everyDays = 7;
static int g_maxRecords = 32768;
static int g_records;
static int g_sequence;
static int g_lastDay = -1;
static int g_lastYear = -1;
static BYTE** g_lastBegin;
static int g_lastCount;
static BYTE* g_pendingResourceBegin;
static int g_pendingResourceCount;
static unsigned long long g_pendingResourceFingerprint;
static int g_resourceStableFrames;
static BYTE** g_lastCapturedPeopleBegin;
static int g_lastCapturedPeopleCount;
static unsigned long long g_lastCapturedResourceFingerprint;
static int g_lastProbeStage = -1;
static char g_sessionId[33];

typedef struct FacilityCandidate {
    uintptr_t identity;
    int buildingType;
    int buildingSubtype;
    bool finished;
    bool goingAway;
    unsigned fieldMask;
    float production;
    float pollution;
    float waterAmount;
    float waterCapacity;
    float waterQuality;
    float sewageAmount;
    float sewageCapacity;
    float sewageQuality;
} FacilityCandidate;

static FacilityCandidate* g_facilities;
static BYTE** g_facilityBegin;
static int g_facilityCount;
static int g_facilityCaptureIndex;
static int g_facilityWriteIndex;
static int g_facilitySequence;
static int g_facilityYear;
static int g_facilityDay;
static int g_lastFacilityYear = -1;
static int g_lastFacilityDay = -1;
static BYTE** g_lastFacilityBegin;
static int g_lastFacilityCount;
static bool g_lastFacilityRejected;
static uintptr_t* g_seenFacilities;
static size_t g_seenFacilityCapacity;
static bool g_facilityWriting;

static bool WriteLine(const char* line)
{
    if (g_output == INVALID_HANDLE_VALUE || g_records >= g_maxRecords) return false;
    TsmWrite(g_output, line, (int)strlen(line));
    TsmWrite(g_output, "\r\n", 2);
    g_records++;
    return true;
}

static void FlushReport(void)
{
    if (g_output != INVALID_HANDLE_VALUE) FlushFileBuffers(g_output);
}

static unsigned ExeTimestamp(void)
{
    if (!ReadablePtr(g_exeBase, sizeof(IMAGE_DOS_HEADER))) return 0;
    const IMAGE_DOS_HEADER* dos = (const IMAGE_DOS_HEADER*)g_exeBase;
    if (dos->e_magic != IMAGE_DOS_SIGNATURE || dos->e_lfanew <= 0) return 0;
    const BYTE* ntAddress = g_exeBase + dos->e_lfanew;
    if (!ReadablePtr(ntAddress, sizeof(IMAGE_NT_HEADERS64))) return 0;
    const IMAGE_NT_HEADERS64* nt = (const IMAGE_NT_HEADERS64*)ntAddress;
    if (nt->Signature != IMAGE_NT_SIGNATURE) return 0;
    return nt->FileHeader.TimeDateStamp;
}

static BYTE* GameState(void)
{
    BYTE* gameState = g_exeBase + RVA_GAME_STATE;
    return ReadablePtr(gameState, 0x600) ? gameState : NULL;
}

static void SetProbeStage(int stage, const char* name)
{
    if (stage == g_lastProbeStage) return;
    char line[256];
    _snprintf_s(line, sizeof(line), _TRUNCATE,
        "{\"schema_version\":4,\"record_type\":\"probe_status\",\"stage\":\"%s\"}",
        name);
    if (WriteLine(line)) {
        g_lastProbeStage = stage;
        FlushReport();
    }
}

static int People(BYTE*** outBegin)
{
    BYTE*** vector = (BYTE***)(g_exeBase + RVA_PERSON_VECTOR);
    if (!ReadablePtr(vector, 16)) return 0;
    BYTE** begin = (BYTE**)vector[0];
    BYTE** end = (BYTE**)vector[1];
    if (!begin || end < begin) return 0;
    size_t bytes = (size_t)((BYTE*)end - (BYTE*)begin);
    if (bytes % sizeof(void*)) return 0;
    size_t count = bytes / sizeof(void*);
    if (count == 0 || count > 500000) return 0;
    *outBegin = begin;
    return (int)count;
}

static bool Finite(float value)
{
    return value == value && value < 3.0e38f && value > -3.0e38f;
}

static unsigned long long HashByte(unsigned long long hash, BYTE value)
{
    return (hash ^ value) * 1099511628211ull;
}

static bool ResourceVector(BYTE** outBegin, int* outCount)
{
    BYTE** vector = (BYTE**)(g_exeBase + RVA_RESOURCE_VECTOR);
    if (!ReadablePtr(vector, 16)) return false;
    BYTE* begin = vector[0];
    BYTE* end = vector[1];
    if (!begin || !end || end < begin) return false;
    size_t bytes = (size_t)(end - begin);
    if (bytes % RESOURCE_STRIDE) return false;
    size_t count = bytes / RESOURCE_STRIDE;
    if (count < 1 || count > RESOURCE_MAX_RECORDS) return false;
    if (!ReadablePtr(begin, bytes)) return false;
    *outBegin = begin;
    *outCount = (int)count;
    return true;
}

static bool ResourceToken(const BYTE* record, char* token, size_t tokenSize);
static bool RollReportForward(void);

static bool BuildingVector(BYTE*** outBegin, int* outCount)
{
    BYTE*** vector = (BYTE***)(g_exeBase + RVA_BUILDING_VECTOR);
    if (!ReadablePtr(vector, 16)) return false;
    BYTE** begin = vector[0];
    BYTE** end = vector[1];
    if (!begin || !end || end < begin) return false;
    size_t bytes = (size_t)((BYTE*)end - (BYTE*)begin);
    if (bytes % sizeof(void*)) return false;
    size_t count = bytes / sizeof(void*);
    if (count < 1 || count > FACILITY_MAX_RECORDS || !ReadablePtr(begin, bytes)) return false;
    *outBegin = begin;
    *outCount = (int)count;
    return true;
}

static void ResetFacilityCapture(void)
{
    if (g_facilities) HeapFree(GetProcessHeap(), 0, g_facilities);
    if (g_seenFacilities) HeapFree(GetProcessHeap(), 0, g_seenFacilities);
    g_facilities = NULL;
    g_seenFacilities = NULL;
    g_seenFacilityCapacity = 0;
    g_facilityBegin = NULL;
    g_facilityCount = 0;
    g_facilityCaptureIndex = 0;
    g_facilityWriteIndex = 0;
    g_facilitySequence = 0;
    g_facilityWriting = false;
}

static bool RememberFacility(BYTE* building)
{
    if (!building || !g_seenFacilities || !g_seenFacilityCapacity) return false;
    uintptr_t identity = (uintptr_t)building;
    size_t index = (size_t)((identity >> 4) ^ (identity >> 21)) & (g_seenFacilityCapacity - 1);
    for (size_t probe = 0; probe < g_seenFacilityCapacity; probe++) {
        uintptr_t* slot = &g_seenFacilities[(index + probe) & (g_seenFacilityCapacity - 1)];
        if (*slot == identity) return false;
        if (*slot == 0) {
            *slot = identity;
            return true;
        }
    }
    return false;
}

static int ResourceRecordIndex(const BYTE* resource, BYTE* begin, int count)
{
    if (!resource || resource < begin) return -1;
    size_t distance = (size_t)(resource - begin);
    if (distance % RESOURCE_STRIDE) return -1;
    size_t index = distance / RESOURCE_STRIDE;
    return index < (size_t)count ? (int)index : -1;
}

static bool CaptureFacility(BYTE* building, BYTE* resourceBegin, int resourceCount,
                            FacilityCandidate* candidate)
{
    if (!RememberFacility(building) || !ReadablePtr(building + BUILDING_TYPE_DESCRIPTOR, 8) ||
        !ReadablePtr(building + BUILDING_COMPLETION, 4) ||
        !ReadablePtr(building + BUILDING_GOING_AWAY, 8)) return false;
    BYTE* descriptor = *(BYTE**)(building + BUILDING_TYPE_DESCRIPTOR);
    if (!descriptor || !ReadablePtr(descriptor + BUILDING_TYPE, 8)) return false;
    int buildingType = *(int*)(descriptor + BUILDING_TYPE);
    int buildingSubtype = *(int*)(descriptor + BUILDING_SUBTYPE);
    float completion = *(float*)(building + BUILDING_COMPLETION);
    if (buildingType < 0 || buildingType > 255 || buildingSubtype < -1 ||
        buildingSubtype > 4096 || !Finite(completion) || completion < 0.0f ||
        completion > 2.0f) return false;

    ZeroMemory(candidate, sizeof(*candidate));
    candidate->identity = (uintptr_t)building;
    candidate->buildingType = buildingType;
    candidate->buildingSubtype = buildingSubtype;
    candidate->finished = completion >= 1.0f;
    candidate->goingAway = *(uintptr_t*)(building + BUILDING_GOING_AWAY) != 0;

    if ((buildingType == 7 || buildingType == 92) &&
        ReadablePtr(building + BUILDING_PRODUCTION, 4)) {
        float value = *(float*)(building + BUILDING_PRODUCTION);
        if (!Finite(value) || value < 0.0f || value > 1.0e12f) return false;
        candidate->production = value;
        candidate->fieldMask |= CANDIDATE_FIELD_PRODUCTION;
    }
    if (buildingType == 2 && ReadablePtr(building + BUILDING_RESIDENTIAL_POLLUTION, 4)) {
        float value = *(float*)(building + BUILDING_RESIDENTIAL_POLLUTION);
        if (!Finite(value) || value < 0.0f || value > 3.0f) return false;
        candidate->pollution = value;
        candidate->fieldMask |= CANDIDATE_FIELD_POLLUTION;
    }

    if (!ReadablePtr(building + BUILDING_STORAGES, 16)) return false;
    BYTE* storageBegin = *(BYTE**)(building + BUILDING_STORAGES);
    BYTE* storageEnd = *(BYTE**)(building + BUILDING_STORAGES + 8);
    if (!storageBegin && !storageEnd) return true;
    if (!storageBegin || !storageEnd || storageEnd < storageBegin) return false;
    size_t storageBytes = (size_t)(storageEnd - storageBegin);
    if (storageBytes % STORAGE_STRIDE || storageBytes / STORAGE_STRIDE > 256 ||
        !ReadablePtr(storageBegin, storageBytes)) return false;

    int waterQualityCount = 0;
    int sewageQualityCount = 0;
    for (size_t storageOffset = 0; storageOffset < storageBytes; storageOffset += STORAGE_STRIDE) {
        BYTE* storage = storageBegin + storageOffset;
        BYTE* slotBegin = *(BYTE**)storage;
        BYTE* slotEnd = *(BYTE**)(storage + 8);
        float capacity = *(float*)(storage + STORAGE_CAPACITY);
        if (!Finite(capacity) || capacity < 0.0f || capacity > 1.0e12f) return false;
        if (!slotBegin && !slotEnd) continue;
        if (!slotBegin || !slotEnd || slotEnd < slotBegin) return false;
        size_t slotBytes = (size_t)(slotEnd - slotBegin);
        if (slotBytes % STORAGE_SLOT_STRIDE || slotBytes / STORAGE_SLOT_STRIDE > 512 ||
            !ReadablePtr(slotBegin, slotBytes)) return false;
        for (size_t slotOffset = 0; slotOffset < slotBytes; slotOffset += STORAGE_SLOT_STRIDE) {
            BYTE* slot = slotBegin + slotOffset;
            BYTE* resource = *(BYTE**)(slot + STORAGE_SLOT_RESOURCE);
            int resourceIndex = ResourceRecordIndex(resource, resourceBegin, resourceCount);
            if (resourceIndex < 0) continue;
            char token[RESOURCE_TOKEN_BYTES + 1];
            if (!ResourceToken(resource, token, sizeof(token))) return false;
            bool water = strcmp(token, "water") == 0;
            bool sewage = strcmp(token, "usagewater") == 0;
            if (!water && !sewage) continue;
            float amount = *(float*)(slot + STORAGE_SLOT_CONTENT);
            float quality = *(float*)(slot + STORAGE_SLOT_QUALITY);
            if (!Finite(amount) || !Finite(quality) || amount < 0.0f ||
                amount > 1.0e12f || quality < 0.0f || quality > 1.5f) return false;
            if (water) {
                candidate->waterAmount += amount;
                candidate->waterCapacity += capacity;
                candidate->waterQuality += quality;
                waterQualityCount++;
                candidate->fieldMask |= CANDIDATE_FIELD_WATER;
            } else {
                candidate->sewageAmount += amount;
                candidate->sewageCapacity += capacity;
                candidate->sewageQuality += quality;
                sewageQualityCount++;
                candidate->fieldMask |= CANDIDATE_FIELD_SEWAGE;
            }
        }
    }
    if (waterQualityCount) candidate->waterQuality /= (float)waterQualityCount;
    if (sewageQualityCount) candidate->sewageQuality /= (float)sewageQualityCount;
    return true;
}

static unsigned long long FacilityFingerprint(const FacilityCandidate* rows, int count)
{
    unsigned long long hash = 1469598103934665603ull;
    for (int i = 0; i < count; i++) {
        const FacilityCandidate* row = &rows[i];
        const BYTE* bytes = (const BYTE*)&row->buildingType;
        for (size_t b = 0; b < sizeof(*row) - sizeof(row->identity); b++)
            hash = HashByte(hash, bytes[b]);
    }
    return hash ^ (unsigned long long)count;
}

static void JsonNumberOrNull(char* output, size_t size, bool present, float value)
{
    if (present) _snprintf_s(output, size, _TRUNCATE, "%.9g", value);
    else strcpy_s(output, size, "null");
}

static void WriteFacilityCandidate(const FacilityCandidate* row, int index)
{
    char production[32], pollution[32], waterAmount[32], waterCapacity[32], waterQuality[32];
    char sewageAmount[32], sewageCapacity[32], sewageQuality[32];
    JsonNumberOrNull(production, sizeof(production),
                     (row->fieldMask & CANDIDATE_FIELD_PRODUCTION) != 0, row->production);
    JsonNumberOrNull(pollution, sizeof(pollution),
                     (row->fieldMask & CANDIDATE_FIELD_POLLUTION) != 0, row->pollution);
    JsonNumberOrNull(waterAmount, sizeof(waterAmount),
                     (row->fieldMask & CANDIDATE_FIELD_WATER) != 0, row->waterAmount);
    JsonNumberOrNull(waterCapacity, sizeof(waterCapacity),
                     (row->fieldMask & CANDIDATE_FIELD_WATER) != 0, row->waterCapacity);
    JsonNumberOrNull(waterQuality, sizeof(waterQuality),
                     (row->fieldMask & CANDIDATE_FIELD_WATER) != 0, row->waterQuality);
    JsonNumberOrNull(sewageAmount, sizeof(sewageAmount),
                     (row->fieldMask & CANDIDATE_FIELD_SEWAGE) != 0, row->sewageAmount);
    JsonNumberOrNull(sewageCapacity, sizeof(sewageCapacity),
                     (row->fieldMask & CANDIDATE_FIELD_SEWAGE) != 0, row->sewageCapacity);
    JsonNumberOrNull(sewageQuality, sizeof(sewageQuality),
                     (row->fieldMask & CANDIDATE_FIELD_SEWAGE) != 0, row->sewageQuality);
    char line[2048];
    _snprintf_s(line, sizeof(line), _TRUNCATE,
        "{\"schema_version\":4,\"record_type\":\"facility_candidate\",\"sequence\":%d,\"facility_index\":%d,\"building_type\":%d,\"building_subtype\":%d,\"finished\":%s,\"going_away\":%s,\"position_x\":null,\"position_z\":null,\"production\":%s,\"pollution\":%s,\"radiation\":null,\"water_amount\":%s,\"water_capacity\":%s,\"water_quality\":%s,\"sewage_amount\":%s,\"sewage_capacity\":%s,\"sewage_quality\":%s,\"field_mask\":%u}",
        g_facilitySequence, index, row->buildingType, row->buildingSubtype,
        row->finished ? "true" : "false", row->goingAway ? "true" : "false",
        production, pollution, waterAmount, waterCapacity, waterQuality,
        sewageAmount, sewageCapacity, sewageQuality, row->fieldMask);
    WriteLine(line);
}

static bool StartFacilityCapture(BYTE** begin, int count, int year, int day)
{
    if (g_records + count + 3 > g_maxRecords && !RollReportForward()) return false;
    ResetFacilityCapture();
    size_t bytes = sizeof(FacilityCandidate) * (size_t)count;
    g_facilities = (FacilityCandidate*)HeapAlloc(GetProcessHeap(), HEAP_ZERO_MEMORY, bytes);
    size_t seenCapacity = 1;
    while (seenCapacity < (size_t)count * 2) seenCapacity <<= 1;
    g_seenFacilities = (uintptr_t*)HeapAlloc(
        GetProcessHeap(), HEAP_ZERO_MEMORY, seenCapacity * sizeof(uintptr_t));
    if (!g_facilities || !g_seenFacilities) {
        ResetFacilityCapture();
        return false;
    }
    g_seenFacilityCapacity = seenCapacity;
    g_facilityBegin = begin;
    g_facilityCount = count;
    g_facilityYear = year;
    g_facilityDay = day;
    g_facilitySequence = ++g_sequence;
    return true;
}

static bool FacilityVectorUnchanged(void)
{
    BYTE** begin = NULL;
    int count = 0;
    return BuildingVector(&begin, &count) && begin == g_facilityBegin && count == g_facilityCount;
}

static bool CaptureFacilityBatch(BYTE* resourceBegin, int resourceCount)
{
    if (!FacilityVectorUnchanged()) return false;
    ULONGLONG started = GetTickCount64();
    int handled = 0;
    while (g_facilityCaptureIndex < g_facilityCount && handled < FACILITY_PER_FRAME) {
        BYTE* building = g_facilityBegin[g_facilityCaptureIndex];
        if (!CaptureFacility(building, resourceBegin, resourceCount,
                             &g_facilities[g_facilityCaptureIndex])) return false;
        g_facilityCaptureIndex++;
        handled++;
        if (handled > 0 && GetTickCount64() - started >= 1) break;
    }
    if (g_facilityCaptureIndex < g_facilityCount) return true;
    if (!FacilityVectorUnchanged()) return false;
    unsigned long long fingerprint = FacilityFingerprint(g_facilities, g_facilityCount);
    char line[768];
    _snprintf_s(line, sizeof(line), _TRUNCATE,
        "{\"schema_version\":4,\"record_type\":\"facility_candidate_snapshot\",\"sequence\":%d,\"year\":%d,\"day\":%d,\"facility_count\":%d,\"candidate_contract_version\":1,\"collection_fingerprint\":\"%016llx\"}",
        g_facilitySequence, g_facilityYear, g_facilityDay, g_facilityCount, fingerprint);
    if (!WriteLine(line)) return false;
    g_facilityWriting = true;
    FlushReport();
    return true;
}

static bool WriteFacilityBatch(void)
{
    ULONGLONG started = GetTickCount64();
    int handled = 0;
    while (g_facilityWriteIndex < g_facilityCount && handled < FACILITY_PER_FRAME) {
        WriteFacilityCandidate(&g_facilities[g_facilityWriteIndex], g_facilityWriteIndex);
        g_facilityWriteIndex++;
        handled++;
        if (handled > 0 && GetTickCount64() - started >= 1) break;
    }
    FlushReport();
    if (g_facilityWriteIndex < g_facilityCount) return true;
    char line[384];
    _snprintf_s(line, sizeof(line), _TRUNCATE,
        "{\"schema_version\":4,\"record_type\":\"facility_candidate_complete\",\"sequence\":%d,\"facility_count\":%d}",
        g_facilitySequence, g_facilityCount);
    if (!WriteLine(line)) return false;
    FlushReport();
    g_lastFacilityYear = g_facilityYear;
    g_lastFacilityDay = g_facilityDay;
    g_lastFacilityBegin = g_facilityBegin;
    g_lastFacilityCount = g_facilityCount;
    g_lastFacilityRejected = false;
    ResetFacilityCapture();
    return true;
}

static int RejectFacilityCapture(void)
{
    g_lastFacilityYear = g_facilityYear;
    g_lastFacilityDay = g_facilityDay;
    g_lastFacilityBegin = g_facilityBegin;
    g_lastFacilityCount = g_facilityCount;
    g_lastFacilityRejected = true;
    ResetFacilityCapture();
    return 7;
}

static int ProcessFacilityCandidate(BYTE* resourceBegin, int resourceCount, int year, int day)
{
    if (g_facilityWriting) {
        if (!WriteFacilityBatch()) {
            return RejectFacilityCapture();
        }
        return g_facilityWriting ? 5 : 6;
    }
    if (g_facilities) {
        if (!CaptureFacilityBatch(resourceBegin, resourceCount)) {
            return RejectFacilityCapture();
        }
        return g_facilityWriting ? 5 : 4;
    }
    BYTE** begin = NULL;
    int count = 0;
    if (!BuildingVector(&begin, &count)) return 3;
    int absoluteDay = year * 365 + day;
    int lastAbsoluteDay = g_lastFacilityYear * 365 + g_lastFacilityDay;
    bool collectionChanged = begin != g_lastFacilityBegin || count != g_lastFacilityCount;
    bool due = collectionChanged || g_lastFacilityDay < 0 ||
               abs(absoluteDay - lastAbsoluteDay) >= g_everyDays;
    if (!due) return g_lastFacilityRejected ? 7 : 6;
    if (!StartFacilityCapture(begin, count, year, day)) {
        g_facilityBegin = begin;
        g_facilityCount = count;
        g_facilityYear = year;
        g_facilityDay = day;
        return RejectFacilityCapture();
    }
    return 4;
}

static bool ResourceToken(const BYTE* record, char* token, size_t tokenSize)
{
    size_t length = 0;
    while (length < RESOURCE_TOKEN_BYTES && record[length]) length++;
    if (length == 0 || length >= RESOURCE_TOKEN_BYTES || length + 1 > tokenSize) return false;
    for (size_t i = 0; i < length; i++) {
        unsigned char c = record[i];
        if (!(isalnum(c) || c == '_' || c == '-' || c == '.')) return false;
        token[i] = (char)c;
    }
    token[length] = 0;
    return true;
}

static bool SaneResource(const BYTE* record, char* token, size_t tokenSize)
{
    if (!ResourceToken(record, token, tokenSize)) return false;
    int kind = *(const int*)(record + RESOURCE_KIND);
    int family = *(const int*)(record + RESOURCE_FAMILY);
    if (kind < -64 || kind > 64 || family < -1 || family > 255) return false;
    const int offsets[] = {
        RESOURCE_PRICE_RUB, RESOURCE_PRICE_USD, RESOURCE_BASE_RUB, RESOURCE_BASE_USD,
        RESOURCE_MARKET_RUB + RESOURCE_MARKET_SELL,
        RESOURCE_MARKET_RUB + RESOURCE_MARKET_BUY,
        RESOURCE_MARKET_USD + RESOURCE_MARKET_SELL,
        RESOURCE_MARKET_USD + RESOURCE_MARKET_BUY
    };
    for (int offset : offsets) {
        float value = *(const float*)(record + offset);
        if (!Finite(value) || value < 0.0f || value > 1.0e12f) return false;
    }
    return true;
}

static unsigned long long ResourceFingerprint(BYTE* begin, int count)
{
    unsigned long long hash = 1469598103934665603ull;
    for (int i = 0; i < count; i++) {
        const BYTE* record = begin + (size_t)i * RESOURCE_STRIDE;
        char token[RESOURCE_TOKEN_BYTES + 1];
        if (!SaneResource(record, token, sizeof(token))) return 0;
        for (const char* p = token; *p; p++) hash = HashByte(hash, (BYTE)*p);
        hash = HashByte(hash, 0);
    }
    return hash ^ (unsigned long long)count;
}

static void WriteResourceEntry(const BYTE* record, int index, int sequence)
{
    char token[RESOURCE_TOKEN_BYTES + 1];
    if (!ResourceToken(record, token, sizeof(token))) return;
    unsigned classMask = 0;
    for (int i = 0; i < RESOURCE_CLASS_COUNT; i++) {
        float factor = *(const float*)(record + RESOURCE_CLASS_BASE + i * RESOURCE_CLASS_STRIDE);
        if (Finite(factor) && factor > 0.0f) classMask |= 1u << i;
    }
    char line[2048];
    _snprintf_s(
        line, sizeof(line), _TRUNCATE,
        "{\"schema_version\":4,\"record_type\":\"resource_entry\",\"sequence\":%d,\"live_index\":%d,\"source_token\":\"%s\",\"caption_id\":%u,\"resource_kind\":%d,\"transport_class_mask\":%u,\"material_family\":%d,\"finished_price_rub\":%.9g,\"finished_price_usd\":%.9g,\"base_price_rub\":%.9g,\"base_price_usd\":%.9g,\"sell_multiplier_rub\":%.9g,\"buy_multiplier_rub\":%.9g,\"sell_multiplier_usd\":%.9g,\"buy_multiplier_usd\":%.9g}",
        sequence, index, token, *(const unsigned*)(record + RESOURCE_CAPTION_ID),
        *(const int*)(record + RESOURCE_KIND), classMask,
        *(const int*)(record + RESOURCE_FAMILY),
        *(const float*)(record + RESOURCE_PRICE_RUB),
        *(const float*)(record + RESOURCE_PRICE_USD),
        *(const float*)(record + RESOURCE_BASE_RUB),
        *(const float*)(record + RESOURCE_BASE_USD),
        *(const float*)(record + RESOURCE_MARKET_RUB + RESOURCE_MARKET_SELL),
        *(const float*)(record + RESOURCE_MARKET_RUB + RESOURCE_MARKET_BUY),
        *(const float*)(record + RESOURCE_MARKET_USD + RESOURCE_MARKET_SELL),
        *(const float*)(record + RESOURCE_MARKET_USD + RESOURCE_MARKET_BUY));
    WriteLine(line);
}

static void CaptureResourceRegistry(BYTE* begin, int count, int year, int day,
                                    unsigned long long fingerprint)
{
    if (g_records + count + 1 > g_maxRecords) return;
    int sequence = ++g_sequence;
    char line[512];
    _snprintf_s(line, sizeof(line), _TRUNCATE,
        "{\"schema_version\":4,\"record_type\":\"resource_registry\",\"sequence\":%d,\"year\":%d,\"day\":%d,\"resource_count\":%d,\"registry_fingerprint\":\"%016llx\"}",
        sequence, year, day, count, fingerprint);
    if (!WriteLine(line)) return;
    for (int i = 0; i < count; i++)
        WriteResourceEntry(begin + (size_t)i * RESOURCE_STRIDE, i, sequence);
    FlushReport();
}

static bool SanePerson(BYTE* person)
{
    if (!person || !ReadablePtr(person, PERSON_SIZE)) return false;
    float age = *(const float*)(person + PERSON_AGE);
    float education = *(const float*)(person + PERSON_EDUCATION);
    int personClass = *(const int*)(person + PERSON_CLASS);
    if (!Finite(age) || age < 0.0f || age > 200.0f) return false;
    if (!Finite(education) || education < 0.0f || education > 3.0f) return false;
    if (personClass < 0 || personClass > 2) return false;
    for (int i = 0; i < 11; i++) {
        float status = *(const float*)(person + PERSON_STATUS + i * 4);
        if (!Finite(status) || status < 0.0f || status > MAX_STATUS_VALUE) return false;
    }
    float moneySpent = *(const float*)(person + PERSON_MONEY_SPENT);
    return Finite(moneySpent) && moneySpent >= 0.0f && moneySpent <= MAX_MONEY_SPENT;
}

static void WriteSession(void)
{
    char line[2048];
    _snprintf_s(
        line, sizeof(line), _TRUNCATE,
        "{\"schema_version\":4,\"record_type\":\"session\",\"probe_id\":\"org.republic-observatory.tesmio-readonly\",\"probe_version\":\"0.3.0\",\"mode\":\"read_only\",\"loader_api_version\":%u,\"target_game_version\":\"1.1.1.9\",\"executable_timestamp\":%u,\"executable_size\":%llu,\"checked_session_id\":\"%s\",\"game_state_rva\":\"0x9D4F10\",\"person_size\":%u,\"person_vector_rva\":\"0x9E75B8\",\"resource_stride\":832,\"resource_vector_rva\":\"0x9E11C0\",\"building_vector_rva\":\"0x9E6A18\",\"facility_candidate_contract_version\":1,\"facility_contract_version\":null,\"writes_game_state\":false,\"writes_save_data\":false,\"writes_observatory_databases\":false,\"network_access\":false}",
        TSM_API_VERSION, ExeTimestamp(), (unsigned long long)g_exeSize, g_sessionId, PERSON_SIZE);
    WriteLine(line);
}

static bool RollReportForward(void)
{
    if (g_output == INVALID_HANDLE_VALUE) return false;
    LARGE_INTEGER beginning = {};
    if (!FlushFileBuffers(g_output) ||
        !SetFilePointerEx(g_output, beginning, NULL, FILE_BEGIN)) {
        Logf("observatory_probe could not roll its bounded report forward");
        return false;
    }
    if (!SetEndOfFile(g_output)) {
        // The file is still intact. Restore append position so the final
        // bounded failure label cannot overwrite its checked session header.
        LARGE_INTEGER end = {};
        SetFilePointerEx(g_output, end, NULL, FILE_END);
        Logf("observatory_probe could not roll its bounded report forward");
        return false;
    }

    // Person samples are temporary research material. Keep the file bounded by
    // starting a fresh checked report in the same allowlisted file. Force the
    // resource registry to be captured again after its normal stability check.
    ResetFacilityCapture();
    g_records = 0;
    g_sequence = 0;
    g_lastProbeStage = -1;
    g_pendingResourceBegin = NULL;
    g_pendingResourceCount = 0;
    g_pendingResourceFingerprint = 0;
    g_resourceStableFrames = 0;
    g_lastCapturedPeopleBegin = NULL;
    g_lastCapturedPeopleCount = 0;
    g_lastCapturedResourceFingerprint = 0;
    g_lastFacilityYear = -1;
    g_lastFacilityDay = -1;
    g_lastFacilityBegin = NULL;
    g_lastFacilityCount = 0;
    g_lastFacilityRejected = false;
    WriteSession();
    FlushReport();
    if (g_records != 1) return false;
    Logf("observatory_probe rolled its bounded report forward");
    return true;
}

static void WriteSample(BYTE* person, int sequence, int sampleIndex, int vectorIndex, int year, int day)
{
    const float* status = (const float*)(person + PERSON_STATUS);
    char line[4096];
    _snprintf_s(
        line, sizeof(line), _TRUNCATE,
        "{\"schema_version\":4,\"record_type\":\"person_sample\",\"sequence\":%d,\"sample_index\":%d,\"vector_index\":%d,\"year\":%d,\"day\":%d,\"current_building_present\":%s,\"age_years\":%.9g,\"education_level\":%.9g,\"status_happiness\":%.9g,\"status_food\":%.9g,\"status_health\":%.9g,\"status_soviet\":%.9g,\"status_alcohol\":%.9g,\"status_culture\":%.9g,\"status_sport\":%.9g,\"status_religion\":%.9g,\"status_clothing\":%.9g,\"status_electronics\":%.9g,\"status_crime\":%.9g,\"citizen_class\":%d,\"money_spent\":%.9g}",
        sequence, sampleIndex, vectorIndex, year, day,
        *(BYTE**)(person + PERSON_CURRENT_BUILDING) ? "true" : "false",
        *(float*)(person + PERSON_AGE), *(float*)(person + PERSON_EDUCATION),
        status[0], status[1], status[2], status[3], status[4], status[5],
        status[6], status[7], status[8], status[9], status[10],
        *(int*)(person + PERSON_CLASS), *(float*)(person + PERSON_MONEY_SPENT));
    WriteLine(line);
}

static void Snapshot(BYTE** begin, int count, int year, int day)
{
    int wanted = g_samples < count ? g_samples : count;
    if (wanted < 1) return;
    // Keep one spare line for the next readiness label. Reaching the bound is
    // a rollover of temporary telemetry, not the end of the checked session.
    if (g_records + wanted + 2 > g_maxRecords && !RollReportForward()) {
        SetProbeStage(5, "stopped_at_record_limit");
        g_enabled = 0;
        Logf("observatory_probe stopped because its bounded report could not roll forward");
        return;
    }
    int step = count / wanted;
    if (step < 1) step = 1;
    int indices[MAX_SAMPLES];
    BYTE* people[MAX_SAMPLES];
    int valid = 0;
    for (int i = 0; i < wanted; i++) {
        int index = i * step;
        if (!ReadablePtr(begin + index, sizeof(void*))) continue;
        BYTE* person = begin[index];
        if (!SanePerson(person)) continue;
        indices[valid] = index;
        people[valid++] = person;
    }
    int sequence = ++g_sequence;
    char line[512];
    _snprintf_s(line, sizeof(line), _TRUNCATE,
        "{\"schema_version\":4,\"record_type\":\"snapshot\",\"sequence\":%d,\"year\":%d,\"day\":%d,\"population_count\":%d,\"sample_count\":%d}",
        sequence, year, day, count, valid);
    WriteLine(line);
    for (int i = 0; i < valid; i++)
        WriteSample(people[i], sequence, i, indices[i], year, day);
    FlushReport();
}

static void Tick(void)
{
    BYTE* world = GameState();
    if (!world) {
        SetProbeStage(1, "waiting_for_game_state");
        return;
    }
    int day = *(const int*)(world + OFF_DAY);
    int year = *(const int*)(world + OFF_YEAR);
    if (day < 0 || day > 365 || year < 1900 || year > 10000) {
        SetProbeStage(2, "waiting_for_loaded_republic");
        return;
    }
    BYTE** begin = NULL;
    int count = People(&begin);
    if (count <= 0) {
        g_lastCapturedPeopleBegin = NULL;
        g_lastCapturedPeopleCount = 0;
        g_lastCapturedResourceFingerprint = 0;
        SetProbeStage(2, "waiting_for_loaded_republic");
        return;
    }
    BYTE* resourceBegin = NULL;
    int resourceCount = 0;
    bool resourceAvailable = ResourceVector(&resourceBegin, &resourceCount);
    if (resourceAvailable) {
        unsigned long long fingerprint = ResourceFingerprint(resourceBegin, resourceCount);
        if (fingerprint && resourceBegin == g_pendingResourceBegin &&
            resourceCount == g_pendingResourceCount &&
            fingerprint == g_pendingResourceFingerprint) {
            g_resourceStableFrames++;
        } else {
            g_pendingResourceBegin = resourceBegin;
            g_pendingResourceCount = resourceCount;
            g_pendingResourceFingerprint = fingerprint;
            g_resourceStableFrames = 1;
        }
        if (fingerprint && g_resourceStableFrames >= 2 &&
            (begin != g_lastCapturedPeopleBegin ||
             count != g_lastCapturedPeopleCount ||
             fingerprint != g_lastCapturedResourceFingerprint)) {
            CaptureResourceRegistry(resourceBegin, resourceCount, year, day, fingerprint);
            g_lastCapturedPeopleBegin = begin;
            g_lastCapturedPeopleCount = count;
            g_lastCapturedResourceFingerprint = fingerprint;
        }
    }
    int facilityStage = resourceAvailable
        ? ProcessFacilityCandidate(resourceBegin, resourceCount, year, day)
        : 3;
    if (!resourceAvailable) {
        SetProbeStage(8, "checked_report_ready_without_resources");
    } else if (facilityStage == 4) {
        SetProbeStage(4, "facility_candidate_collecting");
    } else if (facilityStage == 5) {
        SetProbeStage(5, "facility_candidate_publishing");
    } else if (facilityStage == 6) {
        SetProbeStage(6, "facility_candidate_ready");
    } else if (facilityStage == 7) {
        SetProbeStage(7, "facility_candidate_rejected");
    } else {
        SetProbeStage(3, "checked_report_ready_without_facilities");
    }
    bool worldChanged = begin != g_lastBegin || count != g_lastCount;
    bool dateChanged = day != g_lastDay || year != g_lastYear;
    int absoluteDay = year * 365 + day;
    int lastAbsoluteDay = g_lastYear * 365 + g_lastDay;
    bool intervalPassed = g_lastDay < 0 || abs(absoluteDay - lastAbsoluteDay) >= g_everyDays;
    if (!worldChanged && (!dateChanged || !intervalPassed)) return;
    g_lastBegin = begin;
    g_lastCount = count;
    g_lastDay = day;
    g_lastYear = year;
    Snapshot(begin, count, year, day);
}

static void h_TerrainRender(void* self, bool a, void* cam1, void* cam2, int b, int c)
{
    o_TerrainRender(self, a, cam1, cam2, b, c);
    if (!g_enabled) return;
    __try { Tick(); }
    __except (FaultFilter("Republic Observatory read-only probe", GetExceptionInformation())) {
        g_enabled = 0;
        Logf("observatory_probe disabled after a guarded read fault");
    }
}

static void ReadSettings(void)
{
    const char* ini = "plugins\\observatory_probe.ini";
    g_enabled = H->configInt(ini, "observatory_probe", "enabled", g_enabled);
    g_samples = H->configInt(ini, "observatory_probe", "samples", g_samples);
    g_everyDays = H->configInt(ini, "observatory_probe", "every_days", g_everyDays);
    g_maxRecords = H->configInt(ini, "observatory_probe", "max_records", g_maxRecords);
    if (g_samples < 1) g_samples = 1;
    if (g_samples > MAX_SAMPLES) g_samples = MAX_SAMPLES;
    if (g_everyDays < 1) g_everyDays = 1;
    if (g_everyDays > 365) g_everyDays = 365;
    if (g_maxRecords < 2048) g_maxRecords = 2048;
    if (g_maxRecords > 40000) g_maxRecords = 40000;
}

extern "C" __declspec(dllexport) unsigned TsmPluginApiVersion(void) { return TSM_API_VERSION; }

extern "C" __declspec(dllexport) int TsmPluginInit(const TsmHost* host, TsmPluginInfo* info)
{
    TsmBind(host);
    info->name = "Republic Observatory read-only probe";
    info->version = "0.3.0";
    ReadSettings();
    unsigned timestamp = ExeTimestamp();
    if (!g_enabled) return 1;
    if (timestamp != TESTED_EXE_TIMESTAMP || g_exeSize != TESTED_EXE_SIZE) {
        Logf("observatory_probe refused unsupported executable (timestamp=%08X size=%llu)",
             timestamp, (unsigned long long)g_exeSize);
        return 1;
    }
    _snprintf_s(g_sessionId, sizeof(g_sessionId), _TRUNCATE, "%08X%016llX",
                GetCurrentProcessId(), (unsigned long long)GetTickCount64());
    g_output = TsmOpenLog(PROBE_FILE);
    if (g_output == INVALID_HANDLE_VALUE) {
        Logf("observatory_probe could not create its fixed telemetry file");
        return 1;
    }
    WriteSession();
    FlushReport();
    return 0;
}

extern "C" __declspec(dllexport) int TsmPluginStart(void)
{
    if (!g_enabled || g_output == INVALID_HANDLE_VALUE) return 1;
    if (!PatchIat(g_exe, DLL_ENGINE, SYM_TERRAIN_RENDER, (void*)h_TerrainRender,
                  (void**)&o_TerrainRender, "C3D_TERRAIN::Render")) {
        Logf("observatory_probe found no chainable render import; no observation started");
        return 1;
    }
    Logf("observatory_probe armed: sampled reads only; no game or save writes");
    return 0;
}

BOOL APIENTRY DllMain(HMODULE, DWORD reason, LPVOID)
{
    if (reason == DLL_PROCESS_DETACH) {
        FlushReport();
        ResetFacilityCapture();
        if (g_output != INVALID_HANDLE_VALUE) {
            CloseHandle(g_output);
            g_output = INVALID_HANDLE_VALUE;
        }
    }
    return TRUE;
}
