export const AircastMark = ({ size }: { size: number }) => (
  <svg width={size} height={size} viewBox="0 0 1024 1024" aria-hidden>
    <defs>
      <linearGradient id="aircast-mark-fill" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stopColor="#2B2B2D" />
        <stop offset="1" stopColor="#1D1D1F" />
      </linearGradient>
    </defs>
    <rect width="1024" height="1024" rx="230" fill="url(#aircast-mark-fill)" />
    <path d="M634.2 170.7A362.5 362.5 0 1 1 262 249.5" fill="none" stroke="#0A84FF" strokeWidth="63" strokeLinecap="round" />
    <circle cx="512" cy="162" r="54" fill="#FF9F0A" />
    <path d="M512 308L691 691L512 607L332 691Z" fill="#FFFFFF" />
  </svg>
);
