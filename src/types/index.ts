export interface NqsSnapshot {
  // $22
  engine_variant: string
  engine_variant_raw: number
  // $24 byte 0
  distance_unit: string
  temp_unit: string
  gearbox_type: string
  nation: string
  clock_format: string
  tpms: string
  oil_temp_sensor: string
  spider_version: string
  // $24 byte 1
  configuration_flag: string
  driver_side: string
  fuel: string
  seat_belt_buzzer: string
  arabian_buzzer: string
  door_key_buzzer: string
  brake_type: string
  oil_pressure_sensor: string
  // raw
  byte0_raw: string
  byte1_raw: string
}

export type FieldKey = keyof Omit<NqsSnapshot, 'engine_variant' | 'engine_variant_raw' | 'byte0_raw' | 'byte1_raw'>

export interface FieldDef {
  key: FieldKey
  label: string
  fieldName: string        // key used in write_config changes map
  options: [string, string] // display labels [0-value, 1-value]
  writeValues: [string, string] // values sent to Rust set_field
  danger?: string          // which writeValue triggers a warning
}

// Engine variant field definition — used with ToggleField like all $24 fields.
// writeValues are the raw byte values sent to Rust set_variant() as strings.
// Matches conn_ECU_RLI_FC entries for ECU 209/325 LOCAL_ID $22.
export const ENGINE_VARIANT_FIELD: Omit<FieldDef, 'key'> & { fieldName: 'engine_variant' } = {
  label: 'Engine Variant',
  fieldName: 'engine_variant',
  options: ['1x Flywheel', '4x Flywheel'],
  writeValues: ['3', '4'],
}

// Engine variant options — values are raw bytes sent as strings to write_config
// Matches conn_ECU_RLI_FC entries for ECU 209/325 LOCAL_ID $22
export const ENGINE_VARIANT_OPTIONS = [
  { value: 3, label: '4.3L V8 1x Flywheel' },
  { value: 4, label: '4.3L V8 4x Flywheel' },
] as const

export type EngineVariantValue = typeof ENGINE_VARIANT_OPTIONS[number]['value']

// All $24 fields in exact byte/bit order
export const BYTE0_FIELDS: FieldDef[] = [
  {
    key: 'distance_unit',
    label: 'Distance',
    fieldName: 'distance_unit',
    options: ['Kilometres', 'Miles'],
    writeValues: ['km', 'miles'],
  },
  {
    key: 'temp_unit',
    label: 'Temperature',
    fieldName: 'temp_unit',
    options: ['°C', '°F'],
    writeValues: ['c', 'f'],
  },
  {
    key: 'gearbox_type',
    label: 'Gearbox',
    fieldName: 'gearbox',
    options: ['F1', 'Manual'],
    writeValues: ['f1', 'manual'],
  },
  {
    key: 'nation',
    label: 'Nation',
    fieldName: 'nation',
    options: ['Europe', 'USA'],
    writeValues: ['europe', 'usa'],
  },
  {
    key: 'clock_format',
    label: 'Clock',
    fieldName: 'clock',
    options: ['24 hours', '12 hours'],
    writeValues: ['24h', '12h'],
  },
  {
    key: 'tpms',
    label: 'TPMS',
    fieldName: 'tpms',
    options: ['Disabled', 'Enabled'],
    writeValues: ['disabled', 'enabled'],
  },
  {
    key: 'oil_temp_sensor',
    label: 'Oil Temp Sensor',
    fieldName: 'oil_temp_sensor',
    options: ['Jaeger', 'Bosch'],
    writeValues: ['jaeger', 'bosch'],
  },
  {
    key: 'spider_version',
    label: 'Body Style',
    fieldName: 'spider',
    options: ['Coupé', 'Spider'],
    writeValues: ['no', 'yes'],
  },
]

export const BYTE1_FIELDS: FieldDef[] = [
  {
    key: 'configuration_flag',
    label: 'Config Flag',
    fieldName: 'configuration_flag',
    options: ['OK', 'Not Configured'],
    writeValues: ['ok', 'not_configured'],
    danger: 'not_configured',
  },
  {
    key: 'driver_side',
    label: 'Driver Side',
    fieldName: 'driver_side',
    options: ['LHD', 'RHD'],
    writeValues: ['lhd', 'rhd'],
  },
  {
    key: 'fuel',
    label: 'Fuel',
    fieldName: 'fuel',
    options: ['Unleaded', 'Unrestricted'],
    writeValues: ['unleaded', 'unrestricted'],
  },
  {
    key: 'seat_belt_buzzer',
    label: 'Seat Belt Buzzer',
    fieldName: 'seat_belt_buzzer',
    options: ['Off', 'On'],
    writeValues: ['disabled', 'enabled'],
  },
  {
    key: 'arabian_buzzer',
    label: 'Arabian Buzzer',
    fieldName: 'arabian_buzzer',
    options: ['Off', 'On'],
    writeValues: ['disabled', 'enabled'],
  },
  {
    key: 'door_key_buzzer',
    label: 'Door + Key Buzzer',
    fieldName: 'door_key_buzzer',
    options: ['Off', 'On'],
    writeValues: ['disabled', 'enabled'],
  },
  {
    key: 'brake_type',
    label: 'Brake Type',
    fieldName: 'brake_type',
    options: ['Steel', 'Ceramic'],
    writeValues: ['steel', 'ceramic'],
  },
  {
    key: 'oil_pressure_sensor',
    label: 'Oil Pressure Sensor',
    fieldName: 'oil_pressure',
    options: ['Normal', 'Kavlico'],
    writeValues: ['normal', 'kavlico'],
  },
]
