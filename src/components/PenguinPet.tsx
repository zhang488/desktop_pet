import { useEffect, useState } from 'react'
import './PenguinPet.css'

type Props = {
  /** 角色情绪：normal 平时 / happy 完成任务 / blink 眨眼瞬间 */
  mood?: 'normal' | 'happy'
}

export default function PenguinPet({ mood = 'normal' }: Props) {
  const [blink, setBlink] = useState(false)

  useEffect(() => {
    let timer: number
    const loop = () => {
      timer = window.setTimeout(() => {
        setBlink(true)
        window.setTimeout(() => setBlink(false), 140)
        loop()
      }, 2200 + Math.random() * 2800)
    }
    loop()
    return () => window.clearTimeout(timer)
  }, [])

  const eyeOpen = !blink
  const isHappy = mood === 'happy'

  return (
    <div className="penguin-wrap">
      <svg
        className="penguin-svg"
        viewBox="0 0 200 300"
        xmlns="http://www.w3.org/2000/svg"
        aria-label="角色：企鹅装小女孩"
      >
        <defs>
          <radialGradient id="bellyGrad" cx="50%" cy="40%" r="60%">
            <stop offset="0%" stopColor="#fffaf0" />
            <stop offset="100%" stopColor="#e9e0d0" />
          </radialGradient>
          <radialGradient id="bodyGrad" cx="50%" cy="30%" r="80%">
            <stop offset="0%" stopColor="#2a2630" />
            <stop offset="100%" stopColor="#15131a" />
          </radialGradient>
          <radialGradient id="cheekGrad" cx="50%" cy="50%" r="50%">
            <stop offset="0%" stopColor="#ffb5c5" stopOpacity="0.85" />
            <stop offset="100%" stopColor="#ffb5c5" stopOpacity="0" />
          </radialGradient>
          <radialGradient id="hoodGrad" cx="40%" cy="35%" r="75%">
            <stop offset="0%" stopColor="#332e3a" />
            <stop offset="100%" stopColor="#0e0c12" />
          </radialGradient>
          <linearGradient id="beakGrad" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="#ffc04d" />
            <stop offset="100%" stopColor="#e08a18" />
          </linearGradient>
        </defs>

        {/* ============ 身体 ============ */}
        {/* 阴影 */}
        <ellipse cx="100" cy="285" rx="55" ry="6" fill="#000" opacity="0.18" />

        {/* 左脚 */}
        <ellipse cx="80" cy="278" rx="14" ry="7" fill="url(#beakGrad)" />
        <ellipse cx="80" cy="276" rx="14" ry="7" fill="#ffc04d" />
        {/* 右脚 */}
        <ellipse cx="120" cy="278" rx="14" ry="7" fill="url(#beakGrad)" />
        <ellipse cx="120" cy="276" rx="14" ry="7" fill="#ffc04d" />

        {/* 身体外壳（企鹅装黑色） */}
        <path
          d="M 50 230
             Q 45 195 60 185
             L 140 185
             Q 155 195 150 230
             Q 150 275 100 275
             Q 50 275 50 230 Z"
          fill="url(#bodyGrad)"
        />
        {/* 白色肚子 */}
        <ellipse cx="100" cy="232" rx="32" ry="38" fill="url(#bellyGrad)" />

        {/* 左手 */}
        <ellipse cx="52" cy="225" rx="10" ry="22" fill="url(#bodyGrad)"
          transform="rotate(-8 52 225)" />
        {/* 右手 */}
        <ellipse cx="148" cy="225" rx="10" ry="22" fill="url(#bodyGrad)"
          transform="rotate(8 148 225)" />

        {/* ============ 项圈 ============ */}
        <rect x="62" y="173" width="76" height="11" rx="2" fill="#1a1820" />
        {/* 格子图案 */}
        {[0, 1, 2, 3, 4, 5, 6, 7, 8].map((i) => {
          const colors = ['#e2384a', '#ffffff', '#1a1820']
          return (
            <rect
              key={i}
              x={64 + i * 8.5}
              y={175}
              width="7"
              height="7"
              fill={colors[i % 3]}
              opacity="0.92"
            />
          )
        })}
        {/* 银色别针挂坠 */}
        <g transform="translate(100 188)">
          <ellipse cx="0" cy="0" rx="6" ry="10" fill="none" stroke="#c8ccd1" strokeWidth="2.2" />
          <line x1="0" y1="-7" x2="0" y2="10" stroke="#dadde2" strokeWidth="2.2" />
          <circle cx="0" cy="-9" r="2" fill="#a8acb2" />
        </g>

        {/* ============ 头部 ============ */}
        {/* 呆毛 */}
        <path
          d="M 100 15
             Q 96 5 99 2
             Q 102 0 103 6
             Q 105 12 100 18 Z"
          fill="#15131a"
        />

        {/* 企鹅头套主体（大圆） */}
        <ellipse cx="100" cy="85" rx="72" ry="68" fill="url(#hoodGrad)" />

        {/* 头套高光 */}
        <ellipse cx="75" cy="55" rx="22" ry="14" fill="#fff" opacity="0.06" />

        {/* 企鹅头套左眼装饰（白圈+黑点） */}
        <circle cx="62" cy="62" r="13" fill="#f8f4ea" />
        <circle cx="62" cy="62" r="3.2" fill="#15131a" />
        {/* 右眼装饰 */}
        <circle cx="138" cy="62" r="13" fill="#f8f4ea" />
        <circle cx="138" cy="62" r="3.2" fill="#15131a" />

        {/* 企鹅嘴（黄色三角，朝下） */}
        <path
          d="M 85 78 L 115 78 L 100 100 Z"
          fill="url(#beakGrad)"
          stroke="#b07014"
          strokeWidth="0.8"
        />
        {/* 嘴上高光 */}
        <path d="M 88 80 L 108 80 L 100 86 Z" fill="#ffd97a" opacity="0.7" />

        {/* ============ 脸 ============ */}
        {/* 脸（肤色）—— 在头套下方露出来 */}
        <ellipse cx="100" cy="135" rx="34" ry="28" fill="#fce8d4" />

        {/* 头发：齐刘海 */}
        <path
          d="M 66 118
             Q 70 108 84 110
             L 116 110
             Q 130 108 134 118
             Q 132 132 124 134
             Q 116 122 110 130
             Q 104 122 96 132
             Q 90 122 84 132
             Q 76 122 72 132
             Q 68 130 66 118 Z"
          fill="#2a2530"
        />

        {/* 两侧头发 */}
        <path
          d="M 66 118 Q 60 145 68 160 Q 72 150 72 130 Z"
          fill="#2a2530"
        />
        <path
          d="M 134 118 Q 140 145 132 160 Q 128 150 128 130 Z"
          fill="#2a2530"
        />

        {/* 发夹（左侧蓝白发卡，两条交叉） */}
        <g transform="translate(72 122) rotate(-20)">
          <rect x="-7" y="-1.2" width="14" height="2.4" rx="0.5" fill="#4ba0c0" />
          <rect x="-7" y="-1.2" width="6" height="2.4" rx="0.5" fill="#f0f0f0" />
        </g>
        <g transform="translate(72 127) rotate(20)">
          <rect x="-7" y="-1.2" width="14" height="2.4" rx="0.5" fill="#4ba0c0" />
          <rect x="-7" y="-1.2" width="6" height="2.4" rx="0.5" fill="#f0f0f0" />
        </g>

        {/* ============ 五官 ============ */}
        {/* 左眼 */}
        <g>
          {eyeOpen ? (
            <>
              <ellipse cx="85" cy="138" rx="6.5" ry="8.5" fill="#fffefb" />
              <ellipse cx="85" cy="139" rx="5.2" ry="7" fill="#5a7196" />
              <circle cx="85" cy="140" r="3.5" fill="#1a1820" />
              <circle cx="83.5" cy="137.5" r="1.6" fill="#fff" />
              <circle cx="86.5" cy="141.5" r="0.8" fill="#fff" opacity="0.6" />
            </>
          ) : (
            <path
              d="M 78 138 Q 85 142 92 138"
              stroke="#1a1820"
              strokeWidth="2"
              fill="none"
              strokeLinecap="round"
            />
          )}
        </g>
        {/* 右眼 */}
        <g>
          {eyeOpen ? (
            <>
              <ellipse cx="115" cy="138" rx="6.5" ry="8.5" fill="#fffefb" />
              <ellipse cx="115" cy="139" rx="5.2" ry="7" fill="#5a7196" />
              <circle cx="115" cy="140" r="3.5" fill="#1a1820" />
              <circle cx="113.5" cy="137.5" r="1.6" fill="#fff" />
              <circle cx="116.5" cy="141.5" r="0.8" fill="#fff" opacity="0.6" />
            </>
          ) : (
            <path
              d="M 108 138 Q 115 142 122 138"
              stroke="#1a1820"
              strokeWidth="2"
              fill="none"
              strokeLinecap="round"
            />
          )}
        </g>

        {/* 睫毛 */}
        <path d="M 78 134 Q 82 131 88 132" stroke="#1a1820" strokeWidth="1.4" fill="none" strokeLinecap="round" />
        <path d="M 112 132 Q 118 131 122 134" stroke="#1a1820" strokeWidth="1.4" fill="none" strokeLinecap="round" />

        {/* 腮红 */}
        <ellipse cx="78" cy="152" rx="7" ry="4.5" fill="url(#cheekGrad)" />
        <ellipse cx="122" cy="152" rx="7" ry="4.5" fill="url(#cheekGrad)" />

        {/* 嘴 */}
        {isHappy ? (
          <path
            d="M 92 154 Q 100 166 108 154 Q 100 162 92 154 Z"
            fill="#d94560"
          />
        ) : (
          <path
            d="M 93 155 Q 100 162 107 155 Q 100 159 93 155 Z"
            fill="#d94560"
          />
        )}
        {/* 小虎牙 */}
        <path d="M 96 156 L 97 159 L 98 156 Z" fill="#fff" />
      </svg>
    </div>
  )
}
