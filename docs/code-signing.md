# Code signing (SignPath Foundation)

**現状**: SignPath アカウント／secrets は未用意。**署名なし**で開発・仮 Release する。

後でやるとき:

1. SignPath Foundation に OSS 申請
2. GitHub Actions secrets を設定
3. `.github/workflows/release.yml` に署名ステップを追加
4. 以降の正式 Release のみ署名済み ZIP

それまでは未署名ポータブル ZIP で問題ない（SmartScreen 警告は出うる）。
