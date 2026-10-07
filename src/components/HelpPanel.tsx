const HIGHLIGHTED_PINS = new Set([1, 5, 9])

// Standard 16-pin OBD-II connector housing, traced from a reference SVG
// (obd2.svg in the project root). Pins are generated from a shared grid so
// both rows stay aligned; fill/stroke are driven by HIGHLIGHTED_PINS so the
// wired pins (1, 5, 9) stand out.
function ObdConnectorDiagram() {
  const pinSize = 52
  const pinRadius = 6
  const colStart = 736
  const colStep = 66
  const topY = 506
  const bottomY = 622

  const pins = [
    ...Array.from({ length: 8 }, (_, i) => ({ n: i + 1, x: colStart + i * colStep, y: topY })),
    ...Array.from({ length: 8 }, (_, i) => ({
      n: i + 9,
      x: colStart + i * colStep,
      y: bottomY,
    })),
  ]

  return (
    <svg
      className="obd-diagram"
      viewBox="0 0 699 269"
      role="img"
      aria-label="EOBD connector pinout with pins 1, 5, and 9 marked"
    >
      <g transform="translate(-643.13691,-466.905064)">
        <path
          d="M1084.212,440L1301.258,440C1312.088,440 1321.7,447.785 1325.107,459.318L1383.848,658.167C1388.289,673.201 1386.024,689.713 1377.758,702.553C1369.492,715.394 1356.232,723 1342.113,723L703.887,723C689.768,723 676.508,715.394 668.242,702.553C659.976,689.713 657.711,673.201 662.152,658.167L720.893,459.318C724.3,447.785 733.912,440 744.742,440L960.686,440"
          transform="matrix(-0.955263,-0,0,-0.85159,1969.708047,1085.104711)"
          fill="none"
          stroke="var(--border)"
          strokeWidth={5.53}
        />
        <path
          d="M1052,710.405L1052,725.101C1052,729.461 1048.461,733 1044.101,733L941.899,733C937.539,733 934,729.461 934,725.101L934,710.405"
          fill="none"
          stroke="var(--border)"
          strokeWidth={5}
        />
        <path
          d="M1249,578.52L1249,598.682C1249,600.073 1247.918,601.203 1246.584,601.203L737.416,601.203C736.082,601.203 735,600.073 735,598.682L735,578.52C735,577.129 736.082,576 737.416,576L1246.584,576C1247.918,576 1249,577.129 1249,578.52Z"
          transform="matrix(1.001946,0,0,0.960321,-1.429961,24.75358)"
          fill="none"
          stroke="var(--border)"
          strokeWidth={5.1}
        />
        {pins.map(({ n, x, y }) => {
          const highlighted = HIGHLIGHTED_PINS.has(n)
          return (
            <g key={n}>
              <rect
                x={x}
                y={y}
                width={pinSize}
                height={pinSize}
                rx={pinRadius}
                fill={highlighted ? 'var(--accent)' : 'var(--surface-raised)'}
                stroke={highlighted ? 'var(--accent)' : 'var(--text-muted)'}
                strokeWidth={2.5}
              />
              <text
                x={x + pinSize / 2}
                y={y + pinSize / 2}
                textAnchor="middle"
                dominantBaseline="central"
                fontSize={32}
                fontWeight={700}
                fill={highlighted ? 'var(--bg)' : 'var(--text-muted)'}
              >
                {n}
              </text>
            </g>
          )
        })}
      </g>
    </svg>
  )
}

export function HelpPanel() {
  return (
    <div className="help-panel">
      <span className="help-intro">
        Wire the CANable adapter to the vehicle's EOBD connector as follows:
      </span>
      <div className="help-body">
        <div className="help-pinout">
          <div className="help-pinout-row">
            <span className="help-pinout-signal">GND</span>
            <span className="help-pinout-arrow">→</span>
            <span className="help-pinout-pin">Pin 5</span>
          </div>
          <div className="help-pinout-row">
            <span className="help-pinout-signal">CAN H</span>
            <span className="help-pinout-arrow">→</span>
            <span className="help-pinout-pin">Pin 1</span>
          </div>
          <div className="help-pinout-row">
            <span className="help-pinout-signal">CAN L</span>
            <span className="help-pinout-arrow">→</span>
            <span className="help-pinout-pin">Pin 9</span>
          </div>
        </div>
        <ObdConnectorDiagram />
      </div>
      <div className="help-note">
        Disconnect the CANable's onboard 120 Ω termination resistor jumper before connecting —
        the vehicle's CAN bus is already terminated, and leaving it in can cause bus errors.
      </div>
      <div className="help-link">
        Need a CANable adapter?{' '}
        <a href="https://amzn.to/4gpPRsv" target="_blank" rel="noopener noreferrer">
          Buy one on Amazon
        </a>
      </div>
    </div>
  )
}
