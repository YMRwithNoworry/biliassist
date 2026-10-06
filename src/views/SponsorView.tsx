import { useState } from 'react'
import PageHeader from '../components/PageHeader'
import StatusBar from '../components/StatusBar'
import { IconExternalLink, IconQrCode, IconStar, IconZoom } from '../components/icons'
import { api, errorMessage } from '../lib/ipc'

const PURCHASE_URL = 'https://www.ifdian.net/a/Alkut?tab=home'

const FEATURES = [
  '持续维护与问题修复',
  '新功能开发与体验优化',
  '服务器与基础设施开销',
  '文档与使用教程制作',
]

export default function SponsorView() {
  const [zoomed, setZoomed] = useState(false)
  const [error, setError] = useState('')

  const openPurchasePage = async () => {
    setError('')
    try {
      await api.openExternalUrl(PURCHASE_URL)
    } catch (e) {
      setError(errorMessage(e))
    }
  }

  return (
    <>
      <PageHeader title="赞助支持" subtitle="您的支持是持续维护的动力" />

      <div className="view-body">
        {error ? <StatusBar tone="error">{error}</StatusBar> : null}

        <section className="card sponsor-intro">
          <span className="sponsor-intro-icon">
            <IconStar size={30} strokeWidth={1.5} />
          </span>
          <h2 className="section-title" style={{ marginBottom: 0 }}>
            支持 B站账号管理工具
          </h2>
          <p className="sponsor-intro-text">
            感谢您的使用！如果这个工具对您有帮助，欢迎扫码赞助。您的支持会用于修复问题、
            开发新功能，并保证项目长期可用。
          </p>
        </section>

        <section className="card sponsor-qr">
          <div
            className={zoomed ? 'sponsor-qr-frame zoomed' : 'sponsor-qr-frame'}
            onClick={() => setZoomed((value) => !value)}
          >
            <img className="sponsor-qr-image" src="/sponsor-qr.png" alt="赞助二维码" />
          </div>
          <p className="sponsor-hint">使用微信或支付宝扫描上方二维码</p>
          <p className="sponsor-zoom-hint">
            <IconZoom size={14} />
            {zoomed ? '单击缩小' : '单击放大'}
          </p>
        </section>

        <section className="card">
          <button className="sponsor-link" onClick={() => void openPurchasePage()}>
            <span className="sponsor-link-label">
              <IconExternalLink size={17} />
              爱发电购买 Plus 方案
            </span>
            <span className="sponsor-link-url">ifdian.net/a/Alkut</span>
          </button>
        </section>

        <section className="card">
          <h2 className="section-title">
            <IconQrCode size={18} style={{ verticalAlign: '-3px', marginRight: 6 }} />
            您的支持将用于
          </h2>
          <ul className="feature-list">
            {FEATURES.map((feature) => (
              <li className="feature-item" key={feature}>
                <span className="feature-dot" />
                <span>{feature}</span>
              </li>
            ))}
          </ul>
        </section>

        <section className="card">
          <p className="thanks-text">❤️ 感谢您的每一份支持！</p>
        </section>
      </div>
    </>
  )
}
