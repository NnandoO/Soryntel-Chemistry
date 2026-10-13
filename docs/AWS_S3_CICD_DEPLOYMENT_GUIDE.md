# Guía de Despliegue CI/CD a AWS S3 y Enlaces de Descarga

Esta guía explica cómo configurar el flujo automatizado de CI/CD (GitHub Actions) para compilar, probar y empaquetar **CMA Chemistry**, subir automáticamente los instaladores a un bucket de **Amazon Web Services (AWS) S3** y vincularlos a cualquier sitio web o landing page externa.

---

## 1. Crear y Configurar el Bucket en AWS S3

### Paso 1.1: Crear el Bucket
1. Inicie sesión en la consola de AWS: [https://console.aws.amazon.com/s3](https://console.aws.amazon.com/s3).
2. Haga clic en **Create bucket** (Crear bucket).
3. **Bucket name**: Elija un nombre único global (por ejemplo: `cma-chemistry-downloads` o `downloads.miempresa.com`).
4. **AWS Region**: Seleccione la región deseada (ej. `us-east-1` o `sa-east-1`).
5. **Object Ownership**: Deje la opción recomendada: *ACLs disabled (recommended)*.

---

### Paso 1.2: Configurar Acceso de Descarga Pública
Para que los usuarios puedan descargar los binarios directamente desde su página web sin necesidad de credenciales de AWS, configure el acceso público:

1. En la sección **Block Public Access settings for this bucket**:
   - **Desmarque** la casilla *Block all public access*.
   - Marque la casilla de confirmación (*I acknowledge that the current settings might result in this bucket and the objects within becoming public*).
2. Haga clic en **Create bucket**.

---

### Paso 1.3: Agregar la Política de Bucket (Bucket Policy)
1. Ingrese al bucket recién creado y vaya a la pestaña **Permissions** (Permisos).
2. Desplácese hasta **Bucket policy** y haga clic en **Edit**.
3. Pegue la siguiente política JSON (reemplace `NOMBRE_DE_SU_BUCKET` por el nombre real de su bucket):

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "AllowPublicDownloadReleases",
      "Effect": "Allow",
      "Principal": "*",
      "Action": "s3:GetObject",
      "Resource": "arn:aws:s3:::NOMBRE_DE_SU_BUCKET/releases/*"
    }
  ]
}
```
> **Nota de Seguridad**: Esta política solo permite la descarga pública de los archivos dentro de la carpeta `releases/`. Nadie puede listar todo el bucket ni borrar/subir archivos excepto su CI/CD.

4. Haga clic en **Save changes**.

---

### Paso 1.4 (Opcional recomendado): Configurar CORS
Si su página web utiliza scripts frontend (ej. Fetch/Axios) para verificar el tamaño o descargar el archivo:
1. En la pestaña **Permissions**, desplácese hasta **Cross-origin resource sharing (CORS)** y haga clic en **Edit**.
2. Pegue:
```json
[
  {
    "AllowedHeaders": ["*"],
    "AllowedMethods": ["GET", "HEAD"],
    "AllowedOrigins": ["*"],
    "ExposeHeaders": ["Content-Length", "Content-Type"]
  }
]
```
3. Guarde los cambios.

---

## 2. Crear las Credenciales IAM para GitHub Actions

Nunca use la cuenta raíz ni claves de administrador global para el CI/CD.

1. Vaya a la consola de IAM: [https://console.aws.amazon.com/iam](https://console.aws.amazon.com/iam).
2. En el menú izquierdo, haga clic en **Users** (Usuarios) > **Create user** (Crear usuario).
3. **User name**: `github-actions-cma-uploader`.
4. Haga clic en **Next**.
5. En *Permissions options*, elija **Attach policies directly** > **Create policy** (Crear política en nueva pestaña).
6. Seleccione la pestaña **JSON** y pegue:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "AllowS3ReleaseUpload",
      "Effect": "Allow",
      "Action": [
        "s3:PutObject",
        "s3:GetObject",
        "s3:AbortMultipartUpload",
        "s3:ListBucket"
      ],
      "Resource": [
        "arn:aws:s3:::NOMBRE_DE_SU_BUCKET",
        "arn:aws:s3:::NOMBRE_DE_SU_BUCKET/releases/*"
      ]
    }
  ]
}
```
7. Guarde la política con el nombre `CMA-S3-Uploader-Policy`.
8. Regrese a la creación del usuario, busque y asigne la política `CMA-S3-Uploader-Policy`.
9. Finalice la creación del usuario.
10. Ingrese al usuario creado > pestaña **Security credentials** (Credenciales de seguridad).
11. En **Access keys**, haga clic en **Create access key** > Seleccione **Application running outside AWS** > **Next** > **Create access key**.
12. **Copie y guarde**:
    - `Access key ID`
    - `Secret access key`

---

## 3. Configurar los Secrets en GitHub

1. Vaya a su repositorio en GitHub: `https://github.com/SU_USUARIO/Soryntel-Chemistry`.
2. Haga clic en **Settings** > **Secrets and variables** > **Actions**.
3. Haga clic en **New repository secret** y agregue los siguientes 4 secretos:

| Nombre del Secret | Ejemplo de Valor | Descripción |
| --- | --- | --- |
| `AWS_ACCESS_KEY_ID` | `AKIAIOSFODNN7EXAMPLE` | Access key del usuario IAM |
| `AWS_SECRET_ACCESS_KEY` | `wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY` | Secret key del usuario IAM |
| `AWS_S3_BUCKET` | `cma-chemistry-downloads` | Nombre exacto de su bucket S3 |
| `AWS_REGION` | `us-east-1` | Región AWS de su bucket |

*(Opcional: Si utiliza Amazon CloudFront, agregue también `AWS_CLOUDFRONT_DIST_ID`)*.

---

## 4. Estructura de Archivos Subidos a S3

Cada vez que se sube un cambio a `main` o se crea una versión con git tag (ej. `git tag v1.0.0 && git push origin v1.0.0`), el workflow `.github/workflows/release.yml` compilará en paralelo para Linux y Windows, generando y subiendo a S3:

1. **Archivos Versionados:**
   - `s3://NOMBRE_DE_SU_BUCKET/releases/CMA_Chemistry-1.0.0-win64.zip` (Windows)
   - `s3://NOMBRE_DE_SU_BUCKET/releases/CMA_Chemistry-1.0.0-Linux.tar.gz` (Linux)
   - `s3://NOMBRE_DE_SU_BUCKET/releases/CMA_Chemistry-1.0.0-Linux.deb` (Linux Debian/Ubuntu)
2. **Archivos de Enlace Fijo ("latest"):**
   - `s3://NOMBRE_DE_SU_BUCKET/releases/cma-chemistry-latest-windows.zip` (Windows)
   - `s3://NOMBRE_DE_SU_BUCKET/releases/cma-chemistry-latest-linux.tar.gz` (Linux)
   - `s3://NOMBRE_DE_SU_BUCKET/releases/cma-chemistry-latest-linux.deb` (Linux Debian/Ubuntu)

---

## 5. Vincular las Descargas en su Página Web Externa

Para vincular la descarga en su landing page, sitio web corporativo o portal (HTML, React, WordPress, etc.), utilice la URL pública fija de Amazon S3:

### Formato de las URLs:
```
# Windows
https://NOMBRE_DE_SU_BUCKET.s3.REGION.amazonaws.com/releases/cma-chemistry-latest-windows.zip

# Linux
https://NOMBRE_DE_SU_BUCKET.s3.REGION.amazonaws.com/releases/cma-chemistry-latest-linux.tar.gz
https://NOMBRE_DE_SU_BUCKET.s3.REGION.amazonaws.com/releases/cma-chemistry-latest-linux.deb
```

*(Ejemplo real si su bucket es `cma-downloads` en `us-east-1`)*:
- Windows: `https://cma-downloads.s3.us-east-1.amazonaws.com/releases/cma-chemistry-latest-windows.zip`
- Linux tarball: `https://cma-downloads.s3.us-east-1.amazonaws.com/releases/cma-chemistry-latest-linux.tar.gz`
- Linux deb: `https://cma-downloads.s3.us-east-1.amazonaws.com/releases/cma-chemistry-latest-linux.deb`

---

### Ejemplo de Código HTML para su Página Web:

```html
<div class="download-section">
  <h2>Descargar CMA Chemistry</h2>

  <!-- Windows -->
  <div class="download-os">
    <h3>Windows (64-bit)</h3>
    <p>Incluye ejecutable autónomo y todos los catálogos/datos termodinámicos:</p>
    <a href="https://NOMBRE_DE_SU_BUCKET.s3.REGION.amazonaws.com/releases/cma-chemistry-latest-windows.zip" 
       class="btn btn-primary" 
       download>
      🪟 Descargar para Windows (.zip)
    </a>
  </div>

  <!-- Linux -->
  <div class="download-os">
    <h3>Linux (Ubuntu, Debian, Fedora, Arch)</h3>
    <a href="https://NOMBRE_DE_SU_BUCKET.s3.REGION.amazonaws.com/releases/cma-chemistry-latest-linux.tar.gz" 
       class="btn btn-secondary" 
       download>
      📦 Descargar Binario Linux (.tar.gz)
    </a>
    <a href="https://NOMBRE_DE_SU_BUCKET.s3.REGION.amazonaws.com/releases/cma-chemistry-latest-linux.deb" 
       class="btn btn-secondary" 
       download>
      🐧 Descargar Instalador Debian/Ubuntu (.deb)
    </a>
  </div>
</div>
```

---

## 6. Ejecución Manual del Despliegue

También puede disparar la compilación y subida a S3 en cualquier momento sin hacer commits:
1. Vaya a la pestaña **Actions** en su repositorio de GitHub.
2. En el menú izquierdo, seleccione el workflow **Build, Package & Deploy to S3**.
3. Haga clic en el botón desplegable **Run workflow** > Seleccione la rama `main` > **Run workflow**.
