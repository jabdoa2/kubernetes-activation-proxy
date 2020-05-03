# Kubernetes HTTP activator

This proxy is intended to scale complete kubernes namespaces with multiple deployments to zero
However, it might also work for other usecases.
Scale up will be triggered by the first request to any ingress to your namespace.

Alternatively, you might also use Knative which can scale services to zero.
Scale up with a lot of dependent services might take a while as they are activated sequentially.
This solution allow you to scale everything up at the first request to safe some time.

You need to implement the scale up/down logic in some custom service.
This repo only contains the activation proxy.

Usage:

* Start the activation proxy with a callback to your activation service
* On scale to zero of a namespace
  * Patch all ingresses to use the activation-proxy as upstream
  * Scale down all deployments (and statefulsets) in the namespace
* On activation
  * Check the hostname against all known ingresses (passed as post body)
  * Trigger scale up
  * Patch all ingresses back to their original service
  * Return the original service of the ingress (proxy will wait for this)


# License
MIT
